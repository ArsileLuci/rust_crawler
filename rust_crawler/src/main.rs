use std::io;
use std::collections;
use regex::{Regex};
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

extern crate url;
use url::Url;

extern crate murmur3;

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
        static ref RE2: Regex = Regex::new("\\p{Alphabetic}\\w+").unwrap();
        static ref RE3: Regex = Regex::new("href=\"(/?([\\.\\.]/)*[\\w\\.]+(/[\\w\\.]+)*)\"").unwrap();
    }
    //index and repeat protection
    let mut browsed : HashMap<String, u8> = HashMap::new();

    let mut hasher = DefaultHasher::new();
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
        
        for href in RE3.captures_iter(res.as_str()) {
            let doc = Url::parse(link.trim_end()).unwrap();
            proccessing_queue.push_back(doc.join(&href[1]).unwrap().into_string());
        }

        let mut list : Vec<String> = std::vec::Vec::new();
        for content in RE.split(res.as_str()) {
            let trim = content.trim();
            for word in RE2.captures_iter(trim){
                if word[0].len() >= 4 {
                    list.push(word[0].to_lowercase());
                }
            }
        }
        println!("words found{}", list.len());
        if list.len() < 1024 {
            continue;
        }


        list.sort();
        let mut bx = HashBox::new();

        println!("{}", fna);
        //let mut file = File::create(fna.clone()).unwrap();
        for word in list{
            
            let hash = word.hash(&mut hasher);
            bx.add_hash(&word);


            //file.write_all(&en_stemmer.stem(&word).as_bytes()).unwrap();
            //file.write(b" ").unwrap();
        }
        

        count+=1;
        if count >= required_count {
            break;
        }
    }

    return;
}

struct HashBox {
    hash1:u128,
    hash2:u128,
    hash3:u128,
    hash4:u128,
    hash5:u128,
    hash6:u128,
} 

impl HashBox {
    fn get_hash1(hashable: &str) -> u128 {
        let mut h = hashable.chars(); 
        1u128 << (96+hashable.len()%32) |
        1u128 << (64+ (h.nth(0).unwrap() as u32 % 32)) |
        1u128 << (32+ (h.nth_back(0).unwrap() as u32 % 32)) |
        1u128 << ((h.nth(0).unwrap() as u32 % 32))
    }
    fn get_hash2(hashable: &str) -> u128 {
        let mut h = hashable.chars(); 
        1u128 << (96 + (h.nth(0).unwrap() as u128 % 32)) |
        1u128 << (64 + (h.nth(0).unwrap() as u128 % 32)) |
        1u128 << (32 + (h.nth(0).unwrap() as u128 % 32)) |
        1u128 << (h.nth(0).unwrap() as u128 % 32)
    }
    fn get_hash3(hashable: &str) -> u128 {
        let mut h = hashable.chars(); 
        1u128 << (96 + (h.nth_back(0).unwrap() as u128 % 32)) |
        1u128 << (64 + (h.nth_back(0).unwrap() as u128 % 32)) |
        1u128 << (32 + (h.nth_back(0).unwrap() as u128 % 32)) |
        1u128 << (h.nth_back(0).unwrap() as u128 % 32)
    }
    fn get_hash4(hashable: &str) -> u128 {
        murmur3::murmur3_x64_128(&mut hashable.as_bytes(), 4).unwrap() &
        murmur3::murmur3_x64_128(&mut hashable.as_bytes(), 10).unwrap() &
        murmur3::murmur3_x64_128(&mut hashable.as_bytes(), 16).unwrap() &
        murmur3::murmur3_x64_128(&mut hashable.as_bytes(), 22).unwrap() &
        murmur3::murmur3_x64_128(&mut hashable.as_bytes(), 28).unwrap()
    }
    fn get_hash5(hashable: &str) -> u128 {
        0
    }
    fn get_hash6(hashable: &str) -> u128 {
        0
    }

    pub fn new() -> HashBox {
        HashBox {
            hash1 : 0,
            hash2 : 0,
            hash3 : 0,
            hash4 : 0,
            hash5 : 0,
            hash6 : 0
        }
    }

    pub fn match_hashes(&self, hashable : &str) -> bool {
        self.hash1 & HashBox::get_hash1(hashable) != 0 &&
        self.hash2 & HashBox::get_hash2(hashable) != 0 &&
        self.hash3 & HashBox::get_hash3(hashable) != 0 &&
        self.hash4 & HashBox::get_hash4(hashable) != 0
    }

    pub fn add_hash(&mut self, hashable : &str) {
        self.hash1 |= HashBox::get_hash1(hashable);
        self.hash2 |= HashBox::get_hash2(hashable);
        self.hash3 |= HashBox::get_hash3(hashable); 
        self.hash4 |= HashBox::get_hash4(hashable);

        println!("hash1 {} hash2 {} hash3 {} hash4 {}", self.hash1,self.hash2,self.hash3,self.hash4);
    }
}

