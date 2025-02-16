use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::fs;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use trust_dns_proto::op::{Message, Query};
use trust_dns_proto::rr::{Name, RecordType};
use trust_dns_proto::serialize::binary::{BinDecodable, BinEncodable};
use log::{info, error};

#[derive(Deserialize)]
struct Config {
    dns_server: String,
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Инициализация логирования
    env_logger::init();

    // Чтение конфигурационного файла
    let config_content = fs::read_to_string("config.toml")?;
    let config: Config = toml::from_str(&config_content)?;

    // Получаем аргументы командной строки
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mode> [domain]", args[0]);
        std::process::exit(1);
    }
    let mode = &args[1];

    match mode.as_str() {
        "resolve" => {
            if args.len() != 3 {
                eprintln!("Usage: {} resolve <domain>", args[0]);
                std::process::exit(1);
            }
            let domain = &args[2];
            resolve_domain(&config.dns_server, domain).await?;
        }
        "forward" => {
            forward_dns(&config.dns_server, config.port).await?;
        }
        _ => {
            eprintln!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn resolve_domain(dns_server: &str, domain: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Создаем DNS-запрос
    let name = Name::from_ascii(domain)?;
    let query = Query::query(name, RecordType::A);
    let mut message = Message::new();
    message.add_query(query);
    let query_data = message.to_vec()?;

    // Выполняем HTTP-запрос к DNS-серверу
    let client = Client::new();
    let response = client
        .post(dns_server)
        .header("Content-Type", "application/dns-message")
        .body(query_data)
        .send()
        .await?;

    let response_data = response.bytes().await?;
    let response_message = Message::from_vec(&response_data)?;

    // Выводим IP-адреса из ответа
    for answer in response_message.answers() {
        println!("{}", answer);
    }

    Ok(())
}

async fn forward_dns(dns_server: &str, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let socket = UdpSocket::bind(("0.0.0.0", port)).await?;
    let mut buf = [0; 512];

    info!("Forwarding DNS requests on port {}", port);

    loop {
        let (len, addr) = match socket.recv_from(&mut buf).await {
            Ok(result) => result,
            Err(e) => {
                error!("Failed to receive data: {}", e);
                continue;
            }
        };

        info!("Received request from {}", addr);

        let request = match Message::from_vec(&buf[..len]) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to parse DNS request: {}", e);
                continue;
            }
        };

        let query_data = match request.to_vec() {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to serialize DNS request: {}", e);
                continue;
            }
        };

        let client = Client::new();
        let response = match client
            .post(dns_server)
            .header("Content-Type", "application/dns-message")
            .body(query_data)
            .send()
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                error!("Failed to send DoH request: {}", e);
                continue;
            }
        };

        let response_data = match response.bytes().await {
            Ok(data) => data,
            Err(e) => {
                error!("Failed to read DoH response: {}", e);
                continue;
            }
        };

        if let Err(e) = socket.send_to(&response_data, addr).await {
            error!("Failed to send response to {}: {}", addr, e);
        } else {
            info!("Response sent to {}", addr);
        }
    }
}
