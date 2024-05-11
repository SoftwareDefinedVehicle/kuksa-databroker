# kuksa

Kuksa client library.

```toml
[dependencies]
kuksa = { version = "0.1.0", features = ["grpc"] }
```

## Basic operation
```rust
fn main() {
    let mut client = kuksa::grpc::Client::connect("127.0.0.1:55555")?;

    let speed: f32 = client.get_value("Vehicle.Speed")?;
    println!("Current speed: {speed}");
    
    let stream = client.subscribe("Vehicle.Speed")?;
    while let Some(event) = stream.next() {
        match event {
            Ok(notification) => {
                match notification.get_value() {
                    Ok(speed: f32) => println!("speed: {speed}"),
                    Err(err) => println!("error: {err}"),
                }                
            },
            Err(err) => {
                println!(err);
            }
        }
    }
}
```