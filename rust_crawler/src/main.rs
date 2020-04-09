

use std::io::Read;
use std::io;
use std::fs::{File,OpenOptions};

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
    
    let mut index_file : File;
    let mut paths : Vec<String> = Vec::new();
    match OpenOptions::new().append(true).read(true).open("index.txt"){
        Ok(file) => {
            
            index_file = file;
            let mut string: String = String::new();
            index_file.read_to_string(&mut string).unwrap();
            let rows = string.split("\n");
            for row in rows {
                if row == "" {
                    continue;
                }
                let mut split = row.split("|");
                let url = split.next().expect("index corrupted");
                let path = split.next().expect("index corrupted");
                paths.push(path.to_owned());
            }
        }
        Err(_) => {
            index_file = File::create("index.txt").unwrap();
        }
    }
    


    let mut hash_controller = FileHasher::read_from_stash("./hashes.cache", paths);

    std::fs::create_dir("parsed");
    std::fs::create_dir("stemmed");

    let mut crawler = crawler::Crawler::new(index_file);

    loop 
    {
        let mut command = String::new();
        match io::stdin().read_line(&mut command) {
            Ok(_) => {

                println!("{}",command.trim_end().to_lowercase().as_str());
                match command.trim_end().to_lowercase().as_str() {
                    "crawl" => {
                        crawler::crawl(&mut hash_controller, &mut crawler).await;
                    },
                    "query" => {
                        let mut query = String::new(); 
                        io::stdin().read_line(&mut query).unwrap();
                        println!("{:?}", lexem::parse_to_lexem(&query).unwrap());
                        println!("lexem result in: {:?}", hash_controller.look_out_hash(lexem::parse_to_lexem(&query).unwrap()));
                    },
                    "tf-idf" => {
                        let folder = std::fs::read_dir("stemmed");
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
