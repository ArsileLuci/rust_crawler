use crate::index::hash::HashBox;
use crate::index::lexem::Lexem;
use memmap::MmapOptions;
use std::collections::HashMap;
use std::fs::File;
use std::str;
use std::io::Write;
use std::mem;

pub fn get_index(file_path: &str) -> Vec<HashBox> {
    let file = File::open(file_path).unwrap();
    let mmap = unsafe { MmapOptions::new().map(&file).unwrap() };
    let mut objects: Vec<HashBox> = Vec::new();
    let mut array: [u8; 56] = [0; 56];
    mmap.chunks(54).for_each(|x| {
        array.copy_from_slice(x);
        objects.push(unsafe { mem::transmute::<[u8; 56], HashBox>(array) });
        println!("{:?}", objects);
    }); // .for_each(|f|objects.push( unsafe { mem::transmute::<&[u8; 54], HashBox>(f)}));
        //
    objects
}

pub fn load_initial() {}

pub fn query(lx: Lexem) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let map: HashMap<String, Vec<HashBox>> = HashMap::new();

    names
}

pub fn eval(lx: &Lexem, h: &HashBox, mem: Option<&[u8]>) -> bool {
    match lx {
        Lexem::And(l, r) => eval_and(&*l, &*r, h, mem),
        Lexem::Or(l, r) => eval_or(&*l, &*r, h, mem),
        Lexem::Not(lx2) => eval_not(&*lx2, h, mem),
        Lexem::WithFilter(f) => apply_filter(&f, h, mem),
    }
}

fn eval_and(left: &Lexem, right: &Lexem, h: &HashBox, mem: Option<&[u8]>) -> bool {
    let left_result: bool = eval(left, h, mem);
    if !left_result {
        return false;
    }
    left_result && eval(right, h, mem)
}

fn eval_not(lx: &Lexem, h: &HashBox, mem: Option<&[u8]>) -> bool {
    !eval(lx, h, mem)
}

fn eval_or(left: &Lexem, right: &Lexem, h: &HashBox, mem: Option<&[u8]>) -> bool {
    let left_result: bool = eval(left, h, mem);
    if left_result {
        return true;
    }
    left_result || eval(right, h, mem)
}

fn apply_filter(f: &Filter, h: &HashBox, mem: Option<&[u8]>) -> bool {
    match f {
        Filter::EndsWith(ew) => {
            let matched = h.match_ends_with(&ew);
            if matched {
                match mem {
                    None => return matched,
                    Some(memory) => {
                        let slice = &memory[h.get_ptr() as usize..];
                        let row = str::from_utf8(&slice).unwrap();
                        return row.split_whitespace().take(HashBox::size()).any(|x|x.ends_with(ew))
                    }
                }
            }
            return matched;
        }
        Filter::StartsWith(sw) => {
            let matched = h.match_starts_with(&sw);
            if matched {
                match mem {
                    None => return matched,
                    Some(memory) => {
                        let slice = &memory[h.get_ptr() as usize..];
                        let row = str::from_utf8(&slice).unwrap();
                        return row.split_whitespace().take(HashBox::size()).any(|x|x.starts_with(sw))
                    }
                }
            }
            return matched;
        },
        Filter::Word(w) => {
            let matched = h.match_hashes(&w);
            if matched {
                match mem {
                    None => return matched,
                    Some(memory) => {
                        let slice = &memory[h.get_ptr() as usize..];
                        let row = str::from_utf8(&slice).unwrap();
                        return row.split_whitespace().take(HashBox::size()).any(|x|x==w)
                    }
                }
            }
            return matched;
        }
    }
}

#[derive(Debug)]
pub enum Filter {
    StartsWith(String),
    EndsWith(String),
    Word(String),
}
