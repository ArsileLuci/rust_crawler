use std::io;
use std::collections;
use regex::{Regex};
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;

extern crate url;
use url::Url;

extern crate rust_stemmers;
use rust_stemmers::{Algorithm, Stemmer};

extern crate reqwest;

extern crate tokio;

#[macro_use] extern crate lazy_static;

#[tokio::main]
async fn main() {
    loop 
    {
        let mut command = String::new();
        match io::stdin().read_line(&mut command) {
            Ok(_) => {

                println!("{}",command.trim_end().to_lowercase().as_str());
                match command.trim_end().to_lowercase().as_str() {
                    "crawl" => {
                        crawl().await;
                    },
                    "query" => {},
                    "help" => {},
                    _ => {}
                }
            },
            Err(error) => println!("{:?}", error),
        }
    }
}

async fn crawl() {
    println!("Type your link");
    let mut link = String::new();
    match io::stdin().read_line(&mut link){
        Ok(_) => {},
        Err(error) => println!("{:?}", error),
    }
    println!("Type in pages count");
    let mut count_str = String::new();
    match io::stdin().read_line(&mut count_str){
        Ok(_) => {},
        Err(error) => println!("{:?}", error),
    }
    let required_count : u32 = count_str.trim_end().parse().unwrap(); 
    let mut count = 0;

    let client = reqwest::Client::new();

    let mut proccessing_queue : collections::VecDeque<String> = collections::VecDeque::new();
    lazy_static! {
        static ref RE: Regex = Regex::new("<[^>]*>").unwrap();
        static ref RE2: Regex = Regex::new("\\w+").unwrap();
        static ref RE3: Regex = Regex::new("href=\"(/?([\\.\\.]/)*[\\w\\.]+(/[\\w\\.]+)*)\"").unwrap();
    }
    //index and repeat protection
    let mut browsed : HashMap<String, u8> = HashMap::new();


    //
    let en_stemmer = Stemmer::create(Algorithm::English);

    proccessing_queue.push_back(link);
    loop
    {
        let link = proccessing_queue.pop_front().unwrap();

        let mut filename = link.clone();
        filename = filename.replace("/", ".");
        match browsed.get(&filename) {
            Some(_) => continue,
            None => {},
        }

        println!("Crawling from {}", link.trim_end());
        let res = client.get(link.trim_end())
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        let index : usize;

        match filename.find('#') {
            Some(n) => {
                index = n;
            },
            None => {
                index = filename.len();
            }
        }
        browsed.insert(filename.clone(), 0);
        let fna = format!("{}.txt", &filename[8..index].trim_end());
        println!("{}", fna);
        let mut file = File::create(fna).unwrap();
        
        for href in RE3.captures_iter(res.as_str()) {
            let doc = Url::parse(link.trim_end()).unwrap();
            proccessing_queue.push_back(doc.join(&href[1]).unwrap().into_string());
        }



        for content in RE.split(res.as_str()) {
            let trim = content.trim();
            if trim.len() > 0 {
                for word in RE2.captures_iter(trim){
                    file.write_all(&en_stemmer.stem(&word[0].to_lowercase()).as_bytes()).unwrap();
                    file.write(b" ").unwrap();
                    //println!("{}", en_stemmer.stem(&word[0].to_lowercase()));
                }
            }
        }

        count+=1;
        if count >= required_count {
            break;
        }
    }

    return;
}
