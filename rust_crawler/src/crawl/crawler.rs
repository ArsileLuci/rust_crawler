use url::Host::Domain;
use regex::Regex;
use std::collections;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::thread;

use crate::index::fprocessing::*;
use crate::index::hash::{FileHasher, HashBox};
use crate::index::lexem;

extern crate url;
use url::Url;

extern crate rust_stemmers;
use rust_stemmers::{Algorithm, Stemmer};

extern crate robotstxt;

pub async fn crawl(hc: &mut FileHasher) {
    println!("Type your link");
    let mut link = String::new();
    match io::stdin().read_line(&mut link) {
        Ok(_) => {}
        Err(error) => println!("{:?}", error),
    }
    println!("Type in pages count");
    let mut count_str = String::new();
    match io::stdin().read_line(&mut count_str) {
        Ok(_) => {}
        Err(error) => println!("{:?}", error),
    }
    let required_count: u32 = count_str.trim_end().parse().unwrap();

    let index_file;
    match File::open("index.txt") {
        Err(_) => index_file = File::create("index.txt").unwrap(),
        Ok(file) => index_file = file,
    }
    
    let mut crawler = Crawler::new();
    crawler.crawl(hc, required_count, index_file,link).await;

}

struct Crawler<'a> {
    html_tag_regex: Regex,
    word_regex: Regex,
    href_regex: Regex,
    browsed_links: collections::HashSet<String>,
    robots_hm: collections::HashMap<String, robotstxt::RobotFileParser<'a>>,
    en_stemmer: Stemmer,
    ru_stemmer: Stemmer,
    browsed_count: u32,
    http_client: reqwest::Client,
    external_processing_queue: collections::VecDeque<String>,
    internal_processing_queue: collections::VecDeque<String>, 
}

impl Crawler<'_> {
    pub fn new() -> Self {
        Crawler {
            html_tag_regex : Regex::new("<[^>]*>").unwrap(),
            word_regex : Regex::new("\\p{Alphabetic}\\w+").unwrap(),
            //href_regex : Regex::new("href=[\"'](/?([\\.\\.]/)*|(https?://)?[\\w\\.+=\\-;?&]+(/[\\w\\.+\\-=;?&]+)*/?)[\"']").unwrap(),
            href_regex : Regex::new("href=[\"']([/\\..\\w]+)[\"']").unwrap(),
            browsed_links: collections::HashSet::new(),
            robots_hm : collections::HashMap::new(),
            en_stemmer : Stemmer::create(Algorithm::English),
            ru_stemmer : Stemmer::create(Algorithm::Russian),
            browsed_count : 0,
            http_client : reqwest::Client::new(),
            external_processing_queue : collections::VecDeque::new(),
            internal_processing_queue : collections::VecDeque::new(),
        }
    }

    pub async fn crawl<'a>(&'a mut self, hash_controller: &mut FileHasher, count: u32, mut index_file: File, i_link:String) {
        let mut internal_link : String = url::Url::parse(&i_link).unwrap().domain().unwrap().to_string();
        self.internal_processing_queue.push_back(i_link);

        loop {
            let link;
            match self.internal_processing_queue.pop_front() {
                None => {
                    match self.external_processing_queue.pop_front() {
                        None => {
                            println!("not found enough links");
                            break;
                        }
                        Some(external_link) => {
                            link = external_link.clone();
                            internal_link = url::Url::parse(&external_link).unwrap().domain().unwrap().to_string();
                        }
                    }
                }
                Some(w) => {
                    link = w;
                }
            }

            let mut filename = link.clone();
            filename = filename.replace("/", ".").replace("?", ".");
            
            if !self.can_parse(&link).await {
                continue;
            }

            println!("Crawling from {}", link.trim_end());
            let res = self.http_client
                .get(link.trim_end())
                .send()
                .await;
            let response;
            match res {
                Ok(r) => {
                    response = r.text()
                    .await
                    .unwrap();
                },
                Err(_) => continue,
            }

            //println!("{}", ptr);
            let index: usize;
            match filename.find('#') {
                Some(n) => {
                    index = n;
                }
                None => {
                    index = filename.len();
                }
            }
            println!("{}.txt", &filename[8..index].trim_end());
            let fna = format!("{}.txt", &filename[8..index].trim_end());
            index_file
                .write_all(format!("{}:{},\n", link.trim_end(), fna).as_bytes())
                .unwrap();

            for href in self.href_regex.captures_iter(response.as_str()) {
                let doc = Url::parse(link.trim_end()).unwrap();
                let l = doc.join(&href[1]).unwrap().into_string();
                if l.contains(&internal_link){
                    &self.internal_processing_queue.push_back(l);
                }
                else {
                    &self.external_processing_queue.push_back(l);
                }
                
            }

            let mut list: Vec<String> = std::vec::Vec::new();
            for content in self.html_tag_regex.split(response.as_str()) {
                let trim = content.trim();
                for word in self.word_regex.captures_iter(trim) {
                    if word[0].len() >= 3 {
                        list.push(word[0].to_lowercase());
                    }
                }
            }
            println!("words found:{}", list.len());
            if list.len() < 1024 {
                continue;
            }

            list.sort();

            let mut hash_line = vec![HashBox::new(0), HashBox::new(0)];
            let mut hb_index: usize = 1;
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

                hash_line[0].add_hash(&word);
                hash_line[hb_index].add_hash(&word);
                counter += 1;
                let stemmed_word = &self.en_stemmer.stem(&word);
                let buff = word.as_bytes(); //= stemmed_word.as_bytes();
                hb_ptr += buff.len() as u64;
                file.write_all(buff).unwrap();
                file.write(b" ").unwrap();
                hb_ptr += b" ".len() as u64;
            }
            &hash_controller.add(hash_line, &String::from(filename[8..index].trim_end()));
            self.browsed_count += 1;
            if self.browsed_count >= count {
                break;
            }
        }
        println!("ready");
        println!(
            "\"rust\" probably in: {:?}",
            &hash_controller.look_out_hash(lexem::Lexem::WithFilter(Filter::Word("rust".to_string())))
        );

        return;
    }
    async fn can_parse(&mut self, link: &str) -> bool {
        if link.ends_with("css")
        || link.ends_with("jpg")
        || link.ends_with("png")
        || link.ends_with("js")
        || link.ends_with("wasm")
        || link.ends_with("pdf") 
        || link.ends_with("ttf") {
            return false;
        }

        let string_link = link.to_string();

        if self.browsed_links.insert(string_link) {
            let url = url::Url::parse(link).unwrap();
            match url.host() {
                None => return false,
                Some(host) => {
                    match host {
                        Domain(d) => {
                            match self.robots_hm.get(d) {
                                None => {
                                    let robots_url = format!("{}/robots.txt", d);
                                    let robots_request = self.http_client.get(&robots_url)
                                                            .send()
                                                            .await;
                                    let parser;
                                    match robots_request {
                                        Ok(response) => {
                                            parser = robotstxt::RobotFileParser::parse(response.text().await.unwrap());
                                        }
                                        Err(_) => {
                                            parser = robotstxt::RobotFileParser::parse("User-agent: * \nAllow: /"); 
                                        }
                                    }
                                        let result = parser.can_fetch("ars_rusty_crawler", link);
                                        self.robots_hm.insert(d.to_string(), parser);
                                        return result;
                                    },
                                     
                                Some(r_parser) => {
                                        return r_parser.can_fetch("ars_rusty_crawler", link);
                                    }
                                }
                            },
                        _ => {
                            return false;
                        }
                    }

                    }
                }
            }

        
        return false;
    }
}
