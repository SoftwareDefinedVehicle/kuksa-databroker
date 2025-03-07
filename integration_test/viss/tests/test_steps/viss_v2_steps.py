import asyncio
import http
import logging
import socket
import requests
import websocket
import json
import pytest
from pytest_bdd import given, when, then,parsers

from .http_viss_client import HttpVISSClient
from .mqtt_viss_client import MQTTVISSClient
from .websockets_viss_client import WebSocketsVISSClient
from .types import RequestId

# Basic logging
logging.basicConfig(level=logging.DEBUG, format="%(asctime)s - %(levelname)s - %(message)s")
logger = logging.getLogger(__name__)

class ConnectedClients():

    def __init__(self,pytestconfig):
        self.pytestconfig=pytestconfig
        self.clients = {
            'HTTP': HttpVISSClient(pytestconfig),
            'MQTT': MQTTVISSClient(pytestconfig),
            'WebSockets': WebSocketsVISSClient(pytestconfig)
        }
        logger.debug("Creating new ConnectedClients")
        pass

    def httpclient(self):
        return self.clients['HTTP']

    def wsclient(self):
        return self.clients['WebSockets']

    def mqttclient(self):
        return self.clients['MQTT']

    def connect(self):
        self.wsclient().connect()
        # logger.debug(f"Connecting clients...")
        # for key,client in self.clients.items():
        #     if not client.is_connected():
        #         logger.debug(f"Connecting client: {key}")
        #         client.connect()

    def disconnect(self):
        logger.debug(f"Disconnecting connected clients...")
        for key,client in self.clients.items():
            if client.is_connected():
                logger.debug(f"Disconnecting client: {key}")
                client.disconnect()

    def send(self, request_id,message):
        for key,client in self.clients.items():
            if client.is_connected():
                client.send(request_id,message)
                logger.debug(f"Sent message: {message}")

    def find_subscription_id_by_request_id(self, request_id):
        # TODO: Last one wins
        for key,client in self.clients.items():
            if client.is_connected():
                response=client.find_subscription_id_by_request_id(request_id=request_id,)
                logger.debug(f"Found message: {response}")
        return response

    def find_message(self, *, subscription_id=None, request_id=None, action=None):
        for key,client in self.clients.items():
            if client.is_connected():
                response=client.find_message(subscription_id=subscription_id,
                                             request_id=request_id,
                                             action=action)
                logger.debug(f"Found message: {response}")
        # TODO: Last one wins
        return response

    def find_messages(self, *,
                      subscription_id : str = None,
                      request_id : RequestId = None,
                      action : str = None):
        for key,client in self.clients.items():
            if client.is_connected():
                response=client.find_messages(subscription_id=subscription_id,
                                              request_id=request_id,
                                              action=action)
                logger.debug(f"Found message: {response}")
        return response


@pytest.fixture
def request_id():
    return RequestId()

@pytest.fixture
def connected_clients(request,pytestconfig):
    connected_clients = ConnectedClients(pytestconfig)
    #Enforece all the feature to explicitly state whether to connect and how.
    #connected_clients.connect()
    def cleanup():
        connected_clients.disconnect()
    request.addfinalizer(cleanup)
    return connected_clients

@pytest.fixture
def http_client(connected_clients):
    return connected_clients.clients['HTTP']

@pytest.fixture
def ws_client(connected_clients):
    return connected_clients.clients['WebSockets']

@pytest.fixture
def mqtt_client(connected_clients):
    return connected_clients.clients['MQTT']

# TODO: Parameterize to different permissions/scopes/roles
# TODO: Generate actual token
@given("I am authorized", target_fixture="authorization")
def given_authorized():
    global authorization
    authorization={
        "token": "foobar"
    }
    return authorization

@given("the VISS server is running")
def viss_server_running(pytestconfig):
    # TODO: Check that server is running
    pass

@given("the VISS client is connected via HTTP")
def viss_client_connected_via_http(http_client):
    logger.debug("Connecting via HTTP")
    http_client.connect()

@given("the VISS client is connected via WebSocket")
def viss_client_connected_via_websocket(ws_client):
    logger.debug("Connecting via WebSocket")
    ws_client.connect()

@given(parsers.parse("I have a subscription to \"{path}\""), target_fixture="subscription_id")
@when(parsers.parse('I send a subscription request for "{path}"'), target_fixture="subscription_id")
def send_subscribe(connected_clients, request_id, path):
    request = {"action": "subscribe", "path": path, "requestId": request_id.new()}
    connected_clients.send(request_id, request)
    return connected_clients.find_subscription_id_by_request_id(request_id)

@when(parsers.parse('I subscribe to "{path}" using a curvelog filter with maxerr {maxerr} and bufsize {bufsize}'), target_fixture="subscription_id")
def subscribe_filter_curvelog(connected_clients, request_id, path, maxerr, bufsize):
    request = {
        "action": "subscribe",
        "path": path,
        "filter": {
            "type":"curvelog",
            "parameter": {
                    "maxerr": maxerr,
                    "bufsize": bufsize
                }
        }
        ,
        "requestId": request_id.new()
    }
    connected_clients.send(request_id, request)
    return connected_clients.find_subscription_id_by_request_id(request_id)

@when(parsers.parse('I send an unsubscribe request'))
def send_ws_unsubscribe(connected_clients, request_id, subscription_id):
    request = {"action": "unsubscribe", "subscriptionId": subscription_id, "requestId": request_id.new()}
    connected_clients.send(request_id, request)

@when(parsers.parse('I send a read request with path "{path}"'))
def send_read_data_point(connected_clients, request_id, path):
    request = {"action": "get", "path": path, "requestId": request_id.new()}
    connected_clients.send(request_id, request)

@when(parsers.parse('I search "{path}" using a path filter "{filter}"'))
def search_path_filter(connected_clients,request_id,  path, filter):
    request = {
        "action": "get",
        "path": path,
        "filter" : {
            "type": "paths",
            "parameter" : [
                filter
            ]
        },
        "requestId": request_id.new()
    }
    connected_clients.send(request_id, request)

@when(parsers.parse('I search "{path}" using a history filter "{filter}"'))
def search_history_filter(connected_clients,request_id, path, filter):
    request = {
        "action": "get",
        "path": path,
        "filter" : {
            "type": "history",
            "parameter": filter
        },
        "requestId": request_id.new()
    }
    connected_clients.send(request_id, request)

@when(parsers.parse('I search "{path}" using a dynamic metadata filter "{filter}"'))
def search_dynamic_metadata_filter(connected_clients, request_id, path, filter):
    request = {
        "action": "get",
        "path": path,
        "filter" : {
            "type": "dynamic-metadata",
            "parameter": [
                 filter
            ]
        },
        "requestId": request_id.new()
    }
    connected_clients.send(request_id, request)

@when(parsers.parse('I search "{path}" using a static metadata filter "{filter}"'))
def search_static_metadata_filter(connected_clients, request_id, path, filter):
    request = {
        "action": "get",
        "path": path,
        "filter" : {
            "type": "static-metadata",
            "parameter": [
                filter
            ]
        },
        "requestId": request_id.new()
    }
    connected_clients.send(request_id, request)

@when(parsers.parse('I subscribe to "{path}" using a range filter'), target_fixture="subscription_id")
def search_static_range_filter(connected_clients,request_id, path):
    request = {
        "action": "subscribe",
        "path": path,
        "filter" : {
            "type": "range",
            "parameter":
                [
                    {
                        "boundary-op":"lt",
                        "boundary":"50",
                        "combination-op":"OR"
                    },
                    {
                        "boundary-op":"gt",
                        "boundary":"55"
                    }
                ]
        },
        "requestId": request_id.new()
    }
    connected_clients.send(request_id, request)
    return connected_clients.find_subscription_id_by_request_id(request_id)

@when(parsers.parse('I subscribe to "{path}" using a change filter'), target_fixture="subscription_id")
def search_static_change_filter(connected_clients,request_id,  path):
    request = {
        "action": "subscribe",
        "path": path,
        "filter" : {
            "type": "change",
            "parameter": {
                   "logic-op":"gt",
                   "diff":"10"
            }
        },
        "requestId": request_id.new()
    }
    connected_clients.send(request_id, request)
    return connected_clients.find_subscription_id_by_request_id(request_id)

@when(parsers.parse('I send a set request for path "{path}" with the value {value}'))
def send_set(connected_clients, request_id, path, value):
    request = {"action": "set", "path": path, "requestId": request_id.new(), "value": value}
    connected_clients.send(request_id, request)

@then("I should receive a valid read response")
def receive_valid_get_response(connected_clients, request_id):
    response = connected_clients.find_message(request_id=request_id,
                                              action="get")
    assert "action" in response
    assert "requestId" in response
    assert "data" in response

    assert "error" not in response

    assert response["data"]['path'] != None
    assert response["data"]['dp'] != None
    # the value itself may be "None", but the key must exist
    assert 'value' in response["data"]['dp']
    assert response["data"]['dp']['ts'] != None

@then("I should receive a single value from a single node")
def receive_ws_single_value_single_node(connected_clients, request_id):
    response = connected_clients.find_message(request_id)

    assert response["action"] == "get"
    assert response["requestId"] != None
    assert response["data"] != None
    assert response["data"]['path'] != None
    assert response["data"]['dp'] != None
    # the value itself may be "None", but the key must exist
    assert 'value' in response["data"]['dp']
    assert response["data"]['dp']['ts'] != None

@then("I should receive multiple data points")
def receive_ws_read_multiple_datapoints(connected_clients,request_id):
    responses = connected_clients.find_messages(request_id=request_id)
    # TODO: Unpack envelope
    # TODO: Count number of valid "dp" items
    actual_count = len(responses)
    assert actual_count > 1, f"Expected multiple messages but only got {actual_count}"

@then("I should receive a single value from multiple nodes")
def receive_ws_single_value_multiple_nodes(connected_clients,request_id):
    responses = connected_clients.find_messages(request_id=request_id)
    # TODO: Unpack envelope
    # TODO: Count number of valid "dp" items
    # TODO: Assert that each node only has 1 value
    actual_count = len(responses)
    assert actual_count > 1, f"Expected multiple messages but only got {actual_count}"

@then(parsers.parse("I should receive exactly {expected_count:d} data points"))
def receive_ws_read_expected_count(connected_clients,request_id, subscription_id, expected_count):
    messages = connected_clients.find_messages(subscription_id=subscription_id)
    actual_count = len(messages)
    assert actual_count == expected_count, f"Expected {expected_count} messages but got {actual_count} for subscription {subscription_id}"

@then("I should receive an error response")
def receive_any_error_response(connected_clients,request_id):
    response = connected_clients.find_message(request_id=request_id)
    assert "requestId" in response
    assert "error" in response

@then(parsers.parse("I should receive an error response with number {error_code:d} and reason \"{error_reason}\""))
def receive_specific_error_response(connected_clients,request_id,error_code,error_reason):
    response = connected_clients.find_message(request_id=request_id)
    assert "requestId" in response
    assert "error" in response
    assert error_code == response['error']['number'], f"Expected error code '{error_code}' but got '{response['error']['number']}'"
    assert error_reason == response['error']['reason'], f"Expected error reason '{error_reason}' but got '{response['error']['reason']}'"

@then("I should receive a valid subscribe response")
def receive_ws_subscribe(connected_clients,request_id, subscription_id):
    response = connected_clients.find_message(subscription_id=subscription_id,
                                              request_id=request_id,
                                              action="subscribe")
    assert "subscriptionId" in response
    assert "ts" in response
    assert "error" not in response

@then("I should receive a valid subscription response")
def receive_ws_subscribe(connected_clients,request_id, subscription_id):
    # Do not use the current request_id, as it's from the previous
    # request.
    response = connected_clients.find_message(subscription_id=subscription_id,
                                              request_id=None,
                                              action="subscription")
    assert "action" in response
    assert "subscriptionId" in response
    assert "ts" in response
    assert "requestId" not in response
    assert "error" not in response

@then("I should receive a subscribe error event")
def receive_ws_subscribe_error_event(connected_clients,request_id):
    response = connected_clients.find_message(request_id=request_id, action="subscribe")

    assert "requestId" in response
    assert "error" in response
    assert "ts" in response
    assert "subscriptionId" not in response

    # Current implementation
    assert response["error"] == {"number": 404,
                                 "reason": "invalid_path",
                                 "message": "The specified data path does not exist."}

    # TODO: According to spec example:
    # assert response["error"] == {"number": 404,
    #                              "reason": "unavailable_data",
    #                              "message": "The requested data was not found."}

@then("I should receive a set error event")
def receive_ws_set_error_event(connected_clients,request_id):
    response = connected_clients.find_message(request_id=request_id, action="set")

    assert "action" in response
    assert "requestId" in response
    assert "ts" in response
    assert "error" in response

    # Current implementation
    assert response["error"] == {"number": 401,
                                 "reason": "read_only",
                                 "message": "The desired signal cannot be set since it is a read only signal."}

@then("I should receive a valid unsubscribe response")
def receive_ws_unsubscribe(connected_clients,request_id):
    response = connected_clients.find_message(request_id=request_id, action="unsubscribe")
    assert "action" in response
    assert "subscriptionId" in response
    assert "requestId" in response
    assert "ts" in response

    assert "error" not in response

@then("I should receive a valid subscription event")
def receive_ws_subscription(connected_clients,subscription_id,request_id):
    # Ignore the request id here!
    response = connected_clients.find_message(subscription_id=subscription_id,
                                              request_id=None,
                                              action="subscription")

    assert "action" in response
    assert "subscriptionId" in response
    assert "ts" in response
    assert "data" in response

    assert "requestId" not in response

    assert response["data"]['path'] != None
    assert response["data"]['dp'] != None
    # the value itself may be "None", but the key must exist
    assert 'value' in response["data"]['dp']
    assert response["data"]['dp']['ts'] != None

@then("I should receive a valid set response")
def receive_ws_set(connected_clients,request_id):
    response = connected_clients.find_message(request_id=request_id,action="set")

    assert "action" in response
    assert "requestId" in response
    assert "ts" in response

    assert "error" not in response

@then("I should receive a read-only error")
def receive_ws_set_readonly_error(connected_clients,request_id):
    response = connected_clients.find_message(request_id=request_id)

    assert "action" in response
    assert "requestId" in response
    assert "ts" in response

    assert response["action"] == "set"
    assert response["requestId"] != None
    assert response["ts"] != None
    assert response["error"] == {"number": 401,
                                "reason": "read_only",
                                "message": "The desired signal cannot be set since it is a read only signal."}


@then("I should receive a list of server capabilities")
def receive_ws_list_of_server_capabilities(connected_clients,request_id):
    response = connected_clients.find_message(request_id=request_id)

    assert response == {
        "filter": [
            "timebased",
            "change",
            "dynamic_metadata"
        ],
        "transport_protocol": [
            "https",
            "wss"
        ]
    }
