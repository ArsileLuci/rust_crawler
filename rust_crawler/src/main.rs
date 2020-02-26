use std::io;
use std::collections;
use regex::{Regex};
use std::fs::File;
use std::io::prelude::*;
use std::collections::HashMap;

mod index;
use index::hash::{FileHasher, HashBox};
use index::fprocessing::*;
use index::lexem;
extern crate url;
use url::Url;

extern crate rust_stemmers;
use rust_stemmers::{Algorithm, Stemmer};

extern crate reqwest;

extern crate tokio;

#[macro_use] extern crate lazy_static;

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
                        crawl(&mut hash_controller).await;
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

    let mut index_file = File::create("index.txt").unwrap();

    let mut proccessing_queue : collections::VecDeque<String> = collections::VecDeque::new();
    lazy_static! {
        static ref RE: Regex = Regex::new("<[^>]*>").unwrap();
        static ref RE2: Regex = Regex::new("\\p{Alphabetic}\\w+").unwrap();
        static ref RE3: Regex = Regex::new("href=[\"'](/?([\\.\\.]/)*|(https?://)?[\\w\\.+=\\-;?&]+(/[\\w\\.+\\-=;?&]+)*/?)[\"']").unwrap();
    }
    //index and repeat protection
    let mut browsed : HashMap<String, u8> = HashMap::new();
    //
    let en_stemmer = Stemmer::create(Algorithm::English);

    proccessing_queue.push_back(link);
    loop
    {
        let link;
        match proccessing_queue.pop_front() {
            None => {
                println!("not found enough links");
                break;
            }
            Some(w) => {
                link = w;
            }
        }

        let mut filename = link.clone();
        filename = filename.replace("/", ".").replace("?",".");
        if filename.ends_with("css") ||
           filename.ends_with("jpg") ||
           filename.ends_with("png") ||
           filename.ends_with("js") ||
           filename.ends_with("wasm"){
               continue;
        }
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
        //println!("{}", ptr);
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
        println!("{}.txt", &filename[8..index].trim_end());
        let fna = format!("{}.txt", &filename[8..index].trim_end());
        index_file.write_all(format!("{}:{},\n", link.trim_end(), fna).as_bytes()).unwrap();
        
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

        let mut hash_line = vec!(HashBox::new(0), HashBox::new(0));
        let mut hb_index : usize = 1;
        let mut counter = 0;
        let mut hb_ptr = 0;
        let words_per_cluster = HashBox::size();

        let mut file = File::create(fna.clone()).unwrap();
        for word in list {
            if counter >= words_per_cluster {
                counter = 0;
                hb_index += 1;
                hash_line.push(HashBox::new(hb_ptr));
            }
            //println!("word:\"{}\" at index: {} len {}",&word,counter+128*(hb_index-1), &word.len());
            
            hash_line[0].add_hash(&word);
            hash_line[hb_index].add_hash(&word);
            counter += 1;
            let stemmed_word = &en_stemmer.stem(&word);
            let buff = word.as_bytes();//= stemmed_word.as_bytes();
            hb_ptr += buff.len() as u64;
            file.write_all(buff).unwrap();
            file.write(b" ").unwrap();
            hb_ptr += b" ".len() as u64;
        }
        hc.add(hash_line, &String::from(filename[8..index].trim_end()));
        count+=1;
        if count >= required_count {
            break;
        }
    }
    println!("ready");
    println!("\"rust\" probably in: {:?}", hc.look_out_hash(lexem::Lexem::WithFilter(Filter::Word("rust".to_string()))));

    return;
}
