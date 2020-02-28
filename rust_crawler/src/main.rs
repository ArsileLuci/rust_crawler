use std::io;

mod crawl;
use crawl::crawler;

mod index;
use index::hash::{FileHasher, HashBox};
use index::fprocessing::*;
use index::lexem;

extern crate reqwest;

extern crate tokio;

#[tokio::main]
async fn main() {
    let mut hash_controller = FileHasher::new();
    loop 
    {
        let mut command = String::new();
        match io::stdin().read_line(&mut command) {
            Ok(_) => {

                println!("{}",command.trim_end().to_lowercase().as_str());
                match command.trim_end().to_lowercase().as_str() {
                    "crawl" => {
                        crawler::crawl(&mut hash_controller).await;
                    },
                    "query" => {
                        let mut query = String::new(); 
                        io::stdin().read_line(&mut query).unwrap();
                        println!("{:?}", lexem::parse_to_lexem(&query).unwrap());
                        println!("lexem result in: {:?}", hash_controller.look_out_hash(lexem::parse_to_lexem(&query).unwrap()));
                    },
                    _ => {
                        println!("unknown command, try to use crawl and query");
                    }
                }
            },
            Err(error) => println!("{:?}", error),
        }
    }
}
