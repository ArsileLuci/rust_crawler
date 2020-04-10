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

type Table = HashMap<String, u32>;
type TableF = HashMap<String, f64>; 

pub async fn crawl<'a>(hc: &mut FileHasher, crawler: &mut Crawler<'a>) {
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

    
    crawler.crawl(hc, required_count,link).await;

    hc.write_to_file("./hashes.cache", &crawler.keys_order);

}

pub struct Crawler<'a> {
    index_file: File,
    html_tag_regex: Regex,
    word_regex: Regex,
    href_regex: Regex,
    browsed_links: collections::HashSet<String>,
    pub keys_order: Vec<String>,
    robots_hm: collections::HashMap<String, robotstxt::RobotFileParser<'a>>,
    en_stemmer: Stemmer,
    ru_stemmer: Stemmer,
    browsed_count: u32,
    http_client: reqwest::Client,
    external_processing_queue: collections::VecDeque<String>,
    internal_processing_queue: collections::VecDeque<String>, 
    //TF_IDF
    pages_count: u32,
    tf_counts_table:HashMap<String,Table>,
    idf_counts_table: Table,
    tf_idf: HashMap<String, TableF>,
}

impl Crawler<'_> {
    pub fn new(mut index: File) -> Self {
        let mut visited: collections::HashSet<String> = collections::HashSet::new();
        let mut keys = Vec::<String>::new();
        let mut string: String = String::new();
        index.read_to_string(&mut string);
        let rows = string.split("\n");
        let mut paths : Vec<String> = Vec::new();
        for row in rows {
            if row == "" {
                break;
            }
            let mut split = row.split("|");
            let url = split.next().expect("index corrupted");
            let path = split.next().expect("index corrupted");
            visited.insert(url.to_owned());
            keys.push(path.to_owned());
        }

        Crawler {
            index_file : index,
            html_tag_regex : Regex::new("<[^>]*>").unwrap(),
            word_regex : Regex::new("\\p{Alphabetic}\\w+").unwrap(),
            //href_regex : Regex::new("href=[\"'](/?([\\.\\.]/)*|(https?://)?[\\w\\.+=\\-;?&]+(/[\\w\\.+\\-=;?&]+)*/?)[\"']").unwrap(),
            href_regex : Regex::new("href=[\"']([/\\..\\w]+)[\"']").unwrap(),
            browsed_links: visited,
            keys_order: keys,
            robots_hm : collections::HashMap::new(),
            en_stemmer : Stemmer::create(Algorithm::English),
            ru_stemmer : Stemmer::create(Algorithm::Russian),
            browsed_count : 0,
            //TF-IDF
            tf_counts_table : HashMap::new(),
            idf_counts_table : HashMap::new(),
            pages_count : 0,
            tf_idf: HashMap::new(),
            //
            http_client : reqwest::Client::new(),
            external_processing_queue : collections::VecDeque::new(),
            internal_processing_queue : collections::VecDeque::new(),
        }
    }

    pub async fn crawl<'a>(
        &'a mut self, 
        hash_controller: &mut FileHasher, 
        count: u32, 
        i_link:String
    ) 
    {
        let mut internal_link : String = url::Url::parse(&i_link)
                                                        .unwrap()
                                                        .domain()
                                                        .unwrap()
                                                        .to_string();
        self.internal_processing_queue.push_back(i_link);
        let mut crawled_count : u32 = 0;
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
            
            for href in self.href_regex.captures_iter(response.as_str()) {
                let doc = Url::parse(link.trim_end()).expect("error during url parsing");
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

            let mut hash_line = vec![HashBox::new(0), HashBox::new(0)];
            let mut hb_index: usize = 1;
            let mut counter = 0;
            let mut hb_ptr = 0;
            let words_per_cluster = HashBox::size();

            let mut file = File::create(format!("parsed/{}", fna)).unwrap();
            let mut stemmed_file = File::create(format!("stemmed/{}", fna)).unwrap();
            
            let mut tf = Table::new(); 

            for word in list {
                if counter >= words_per_cluster {
                    counter = 0;
                    hb_index += 1;
                    hash_line.push(HashBox::new(hb_ptr));
                }

                hash_line[0].add_hash(&word);
                hash_line[hb_index].add_hash(&word);
                counter += 1;
                let en_stemmed = &self.en_stemmer.stem(&word).to_owned();
                let stemmed_word = &self.ru_stemmer.stem(en_stemmed);
                
                let buff = word.as_bytes();
                hb_ptr += buff.len() as u64;
                //
                file.write_all(buff).unwrap();
                file.write(b" ").unwrap();
                hb_ptr += b" ".len() as u64;
                //
                stemmed_file.write(stemmed_word.as_bytes()).unwrap();
                stemmed_file.write(b" ").unwrap();
                //TF-IDF
                let stemmed_key = stemmed_word.to_string();
                let _get = tf.get(&stemmed_key);
                match _get {
                    None => {
                        let _idf =self.idf_counts_table.get(&stemmed_key);
                        match _idf {
                            None => {
                                self.idf_counts_table.insert(stemmed_key.clone(), 1);
                            }
                            Some(idf) => {
                                self.idf_counts_table.insert(stemmed_key.clone(), idf + 1);
                            }
                        }
                        tf.insert(stemmed_key, 1);
                    }
                    Some(val) => {
                        tf.insert(stemmed_key, val + 1);
                    }
                }
            }
            
            self.tf_counts_table.insert(fna.clone(), tf);
            self.tf_idf.insert(fna.clone(), HashMap::new());
            &self.index_file
                .write_all(format!("{}|{}\n", link.trim_end(), fna).as_bytes())
                .expect("error during writing to index file");
            &hash_controller.add(hash_line, &fna);
            self.keys_order.push(fna);

            self.browsed_count += 1;
            crawled_count += 1;
            self.pages_count+=1;

            if crawled_count >= count {
                break;
            }
        }
        
        let mut table = prettytable::Table::new();
        
        for idf in &self.idf_counts_table {
            let mut row : Vec::<prettytable::Cell> = Vec::new();
            let _idf = (*idf.1 as f64)/(self.tf_counts_table.len() as f64);
            row.push(prettytable::Cell::new(idf.0));
            let idf_string = format!("idf:{}", _idf);
            row.push(prettytable::Cell::new(&idf_string));
            for tf_table in &self.tf_counts_table {
                let tf_idf_table = self.tf_idf.get_mut(tf_table.0).unwrap();
                match tf_table.1.get(idf.0) {
                    None => {
                        tf_idf_table.insert(idf.0.to_string(), 0_f64);
                        row.push(prettytable::Cell::new("tf: 0\ntf-idf: 0"));
                    }
                    Some(value) => {
                        let tf = (*value as f64)/(tf_table.1.len() as f64);
                        let tf_idf = (*value as f64 / (tf_table.1.len() as f64))/(_idf);
                        let string = format!("tf: {:.5}\ntf-idf: {:.3}", tf, tf_idf);
                        tf_idf_table.insert(idf.0.to_string(), tf_idf);
                        row.push(prettytable::Cell::new(&string));
                    }
                }
            }
            table.add_row(prettytable::Row::new(row));
        }

        let mut table_file = File::create("./tf_idf.txt").unwrap();
        table.print(&mut table_file);
        return;
    }
    
    async fn can_parse(&mut self, link: &str) -> bool {
        if link.ends_with("css")
        || link.ends_with("js")
        || link.ends_with("pdf") 
        || link.ends_with("ttf")
        || link.ends_with("wasm")
        || link.ends_with("jpg")
        || link.ends_with("png")
        || link.ends_with("bmp")
        || link.ends_with("ico") {
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

    pub fn search(&self, search_string : &str) {
        let split = search_string.split_whitespace();
        let mut split_vec = Vec::<String>::new();
        for word in split {
            split_vec.push(word.to_owned());
        }
        let mut search_tf_idf : HashMap<String, f64> = HashMap::new();

        for word in &split_vec {
            let ru_stemmed = &self.ru_stemmer.stem(word);
            let stemmed = &self.en_stemmer.stem(ru_stemmed);
            search_tf_idf.insert(stemmed.to_string(), 1_f64 / (split_vec.len() as f64));
        }
        let mut rated = Vec::<(f64,String)>::new();
        for doc_tf_idf in &self.tf_idf {
            let mut used_words = 0;
            let mut search_vec = Vec::<f64>::new();
            let mut doc_vec= Vec::<f64>::new();
            for word in doc_tf_idf.1 {
                match search_tf_idf.get(word.0) {
                    None => {
                        search_vec.push(0_f64);
                        doc_vec.push(*word.1);
                    }
                    Some(tf_idf) => {
                        search_vec.push(*tf_idf);
                        doc_vec.push(*word.1);
                        used_words+=1;
                    }
                }
            }
            if used_words < search_tf_idf.len() {
                for word in &search_tf_idf {
                    match doc_tf_idf.1.get(word.0) {
                        None => {
                            search_vec.push(*word.1);
                            doc_vec.push(0_f64);
                        }
                        Some(_) => {
                        }
                    }
                }
            }
            let doc_array = ndarray::arr1(&doc_vec);
            let rating = ndarray::arr1(&search_vec).dot(&doc_array);
            rated.push((rating, doc_tf_idf.0.to_owned()));
        }
        rated.sort_by(|a,b| a.0.partial_cmp(&b.0).unwrap());
        rated.reverse();
        for item in &rated[0..10] {
            println!("{}:{}",item.0, item.1);
        }
    }
}
