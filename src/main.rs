use std::{
    io::{Error, Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
};

use sha1::Digest;

fn main() -> Result<(), Error> {
    let srv = TcpListener::bind("127.0.0.1:2998")?;

    std::fs::create_dir_all("client")?;
    std::fs::create_dir_all("server")?;

    while let Ok((conn, source)) = srv.accept() {
        println!("connection from {:?}", source);
        match proxy_connection(conn) {
            Ok(_) => println!("connection from {source:?} done"),
            Err(e) => println!("connection from {source:?} error: {e}"),
        }
    }

    Ok(())
}

fn proxy_connection(from_client: TcpStream) -> Result<(), Error> {
    // open server end
    let to_server = TcpStream::connect("127.0.0.1:2794")?;

    // duplicate streams
    let to_client = from_client.try_clone()?;
    let from_server = to_server.try_clone()?;

    let mut client_data = Vec::new();
    let mut server_data = Vec::new();

    let (c, s) = std::thread::scope(|s| {
        let c = s.spawn(|| proxy("client", from_client, to_server, &mut client_data));
        let s = s.spawn(|| proxy("server", from_server, to_client, &mut server_data));
        (c.join().unwrap(), s.join().unwrap())
    });

    // save data to files
    let cf = save_file(&client_data, "client");
    let sf = save_file(&server_data, "server");

    cf?;
    sf?;
    c?;
    s?;

    Ok(())
}

fn proxy(
    src: &'static str,
    mut from: TcpStream,
    mut to: TcpStream,
    buf: &mut Vec<u8>,
) -> Result<(), Error> {
    loop {
        // get data
        let mut b = [0; 1024];

        println!("reading {src}");
        let bread = match from.read(&mut b) {
            Ok(0) => {
                println!("done reading {src}");
                //from.shutdown(Shutdown::Read)?;
                to.shutdown(Shutdown::Read)?;
                break;
            }
            Ok(n) => n,
            Err(e) => {
                println!("error reading {src}: {}", e);
                break;
            }
        };
        let data = &b[..bread];

        // save data
        buf.extend_from_slice(data);

        // send data
        println!("forwarding {src}");
        to.write_all(data)?;
        to.flush()?;
    }

    Ok(())
}

fn save_file(data: &[u8], dir: &'static str) -> Result<(), Error> {
    let hash = sha1::Sha1::digest(&data)
        .iter()
        .fold(String::new(), |mut s, b| {
            s += &format!("{b:x}");
            s
        });

    std::fs::write(format!("{dir}/{hash}"), data)?;
    Ok(())
}
