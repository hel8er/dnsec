use reqwest::Client;
use std::env;
use trust_dns_proto::op::{Message, Query};
use trust_dns_proto::rr::{Name, RecordType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Получаем аргументы командной строки
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <domain>", args[0]);
        std::process::exit(1);
    }
    let domain = &args[1];
    let dns_server = "https://cloudflare-dns.com/dns-query";

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
