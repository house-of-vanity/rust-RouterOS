# rust-RouterOS
### Simple class to interact with RouterOS via ROS API.

Add dependency to `Cargo.toml`
```rust
router_os = { git = "https://github.com/house-of-vanity/rust-RouterOS", branch = "master" }
```
Example `src/main.rs`. Print all DHCP client's MAC addresses.
```rust
extern crate router_os;
use router_os::ApiRos;
use std::{net::TcpStream, process::exit};

fn main() {
    let user = "admin";
    let pass = "admin";
    let mut stream = TcpStream::connect("192.168.88.1:8728").unwrap();
    let mut apiros = ApiRos::new(&mut stream);
    match apiros.login(user.to_string(), pass.to_string()) {
        Ok(_u) => {}
        Err(err) => {
            println!("RouterOS authentication error: {:?}", err);
            exit(0x2)
        }
    };
    let x = match apiros.talk(vec!["/ip/dhcp-server/lease/print".to_string()]) {
        Ok(reply) => reply,
        Err(err) => {
            println!("RouterOS API Error: {:?}", err);
            exit(0x1)
        }
    };
    for i in 0..x.len() - 1 {
        println!("{}", x[i].1["=mac-address"]);
    }
}
```
