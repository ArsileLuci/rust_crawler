use std::io;
use std::collections;
use regex::{Regex};
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;

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
        let mut hash_controller = FileHasher::new();
        match io::stdin().read_line(&mut command) {
            Ok(_) => {

                println!("{}",command.trim_end().to_lowercase().as_str());
                match command.trim_end().to_lowercase().as_str() {
                    "crawl" => {
                        crawl(&mut hash_controller).await;
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

async fn crawl(hc: &mut FileHasher) {
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
        println!("words found:{}", list.len());
        if list.len() < 1024 {
            continue;
        }


        list.sort();

        let mut hash_line = vec!(HashBox::new(), HashBox::new());
        let mut hb_index : usize = 1;
        let mut counter = 0;

        let words_per_claster = 64;

        println!("{}", fna);
        let mut file = File::create(fna.clone()).unwrap();
        for word in list {
            if counter >= words_per_claster {
                counter = 0;
                hb_index += 1;
                hash_line.push(HashBox::new());
            }
            //println!("word:\"{}\" at index: {} len {}",&word,counter+128*(hb_index-1), &word.len());
            
            hash_line[0].add_hash(&word);
            hash_line[hb_index].add_hash(&word);
            counter += 1;

            file.write_all(&en_stemmer.stem(&word).as_bytes()).unwrap();
            file.write(b" ").unwrap();
        }
        hc.add(hash_line, &String::from(filename[8..index].trim_end()));

        count+=1;
        if count >= required_count {
            break;
        }
    }
    println!("ready");
    println!("\"rust\" found in: {:?}", hc.look_out_hash("rust").unwrap());

    return;
}

#[derive(Debug)]
struct HashBox {
    words_hash:u128,
    starts_with_hash:u128,
    ends_with_hash:u128,
    hash4:u128,
    hash5:u128,
    hash6:u128,
} 

impl HashBox {
    fn get_word_hash(hashable: &str) -> u128 {
        let mut h = hashable.bytes(); 
        1u128 << (96+ hashable.len()%32) |
        1u128 << (64+ (h.next().unwrap() as u32 % 32)) |
        1u128 << ((h.next().unwrap() as u32 % 32)) |
        1u128 << (32+ (h.rev().next().unwrap() as u32 % 32))
    }
    fn get_starts_with_hash(hashable: &str) -> u128 {
        let mut h = hashable.bytes(); 
        1u128 << (96 + (h.next().unwrap() as u128 % 32)) |
        1u128 << (64 + (h.next().unwrap() as u128 % 32)) |
        1u128 << (32 + (h.next().unwrap() as u128 % 32)) |
        1u128 << (h.next().unwrap() as u128 % 32)
    }
    fn get_ends_with_hash(hashable: &str) -> u128 {
        let mut h = hashable.bytes().rev(); 
        1u128 << (96 + (h.next_back().unwrap() as u128 % 32)) |
        1u128 << (64 + (h.next_back().unwrap() as u128 % 32)) |
        1u128 << (32 + (h.next_back().unwrap() as u128 % 32)) |
        1u128 << (h.next_back().unwrap() as u128 % 32)
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
            words_hash : 0,
            starts_with_hash : 0,
            ends_with_hash : 0,
            hash4 : 0,
            hash5 : 0,
            hash6 : 0
        }
    }

    pub fn match_hashes(&self, hashable : &str) -> bool {
        let word = HashBox::get_word_hash(hashable);
        let start = HashBox::get_starts_with_hash(hashable);
        let end = HashBox::get_ends_with_hash(hashable);


        (self.words_hash & word) == word &&
        (self.starts_with_hash & start) == start &&
        (self.ends_with_hash & end) == end 
    }

    pub fn add_hash(&mut self, hashable : &str) {
        self.words_hash |= HashBox::get_word_hash(hashable);
        self.starts_with_hash |= HashBox::get_starts_with_hash(hashable);
        self.ends_with_hash |= HashBox::get_ends_with_hash(hashable);

        //println!("words_hash {} starts_with_hash {} ends_with_hash {}", self.words_hash,self.starts_with_hash,self.ends_with_hash);
    }
}

#[derive(Debug)]
struct FileHasher {
    map : HashMap<String, Vec<HashBox>>
}

impl FileHasher {
    pub fn new() -> Self {
        FileHasher {
            map: HashMap::new()
        }
    }
    pub fn add(&mut self, v:Vec<HashBox> , k: &str){
        self.map.insert(String::from(k), v);
    }

    pub fn look_out_hash(&mut self, look_out_str:&str) -> Option<Vec<String>> {
        let mut vec : Vec<String> = Vec::new();
        for item in self.map.iter(){
            if item.1[1..].iter().any(|x|x.match_hashes(look_out_str)) {
                vec.push(item.0.to_string());
            }
        }
        if vec.len() > 0 {
            return Some(vec);
        }

        None
    }
}