use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::fs;
use trust_dns_proto::op::{Message, Query};
use trust_dns_proto::rr::{Name, RecordType};
use trust_dns_proto::serialize::binary::{BinDecodable, BinEncodable};

#[derive(Deserialize)]
struct Config {
    dns_server: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
            // Реализация режима перенаправления
            // Здесь будет код для перенаправления обычных DNS-запросов в DoH
            println!("Forward mode is not implemented yet.");
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
