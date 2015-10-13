///! A program that crawls a webpage and grabs all the links.
///! Then, through some machine learning, it will translate those "old"
///! links and convert them to the new links

extern crate curl;
extern crate html5ever;
extern crate clap;
extern crate edit_distance;

#[macro_use]
extern crate string_cache;
extern crate tendril;

use curl::http;

use html5ever::{parse, one_input};
use html5ever::rcdom::{Element, RcDom, Handle};
use html5ever::tokenizer::Attribute;

use tendril::{SliceExt, StrTendril};
use tendril::fmt::UTF8;

use clap::{Arg, App};

use edit_distance::edit_distance;

struct GetLinks {
    links: Vec<StrTendril>,
    url: String,
    pwd: String
}

impl GetLinks {
    pub fn new(url: String, pwd: String) -> GetLinks {
        GetLinks {
            links: Vec::new(),
            url: url,
            pwd: pwd
        }
    }

    pub fn gen_dom(&self) -> RcDom {
        let resp = http::handle()
            .userpwd(&self.pwd[..])
            .get(&self.url[..])
            .exec().unwrap();


        let input = resp.get_body().to_tendril();
        let input = input.try_reinterpret().unwrap();

        parse(one_input(input), Default::default())
    }

    pub fn find_links(&mut self, handle: Handle) { 
        let node = handle.borrow();

        match node.node {
            Element(ref name, _, ref attrs) => {
                assert!(name.ns == ns!(html));
                if name.local.as_slice() == "a" {
                    let mut href_attr = attrs.iter().filter(|&x| x.name.local.as_slice() == "href").take(1);
                    match href_attr.next() {
                        Some(attr) => {
                            assert!(attr.name.ns == ns!(""));
                            let href_val = attr.value.clone();
                            if !href_val.is_empty()  {
                                let first = &href_val[0..1];
                                if first == "/" || (&href_val).contains(&self.url[..]) {
                                    self.links.push(attr.value.clone());
                                }
                            }
                        },
                        None => {
                            println!("Something went wrong with the attrs next stuff");
                        }
                    }
                }
            }
            _ => {}
        }

        self.links.sort();
        self.links.dedup();

        for child in node.children.iter() {
            self.find_links(child.clone());
        }

    }

    pub fn get_links(&self) -> Vec<StrTendril> { return self.links.clone(); }
    pub fn get_url(&self) -> String { return self.url.clone(); }
    pub fn get_pwd(&self) -> String { return self.pwd.clone(); }
}

fn avg_key_match(i: &str, j: &str) -> f64 {
    let keywords: Vec<&str> = j.split('/').filter(|&x| x != "/").collect();
    let mut cnt: i32 = 0;
    for keyword in keywords.iter() {
        if i.contains(keyword) {
            cnt += 1;
        }
    }

    (cnt as f64 / keywords.len() as f64)
}

fn find_match(u1: &str, u2: &str, url1: &StrTendril, links: Vec<StrTendril>) -> StrTendril {
    let u: &str = if url1.contains(u1) {
        &url1[u1.len()..]
    }else {
        url1
    };

    let mut match_tendril = links[0].clone();
    let l: &str = if links[0].contains(u2) {
        &links[0][u2.len()..]
    }else {
        &links[0]
    };

    let mut edits = avg_key_match(u, l);

    for link in links.iter().skip(1) {
        let l: &str = if link.contains(u2) {
            &link[u2.len()..]
        }else {
            link 
        };

        let e = avg_key_match(u, l); 
        if e > edits { 
            edits = e;
            match_tendril = link.clone();
        }
    }
    println!("\n\n{} == {}", edits, match_tendril);
    match_tendril
}

fn main() {
    let matches = App::new("gen_redirects")
        .version("0.1.0")
        .author("Jason Cardinal <jason.brogrammer@gmail.com>")
        .about("Get the links from a website")
        .arg(Arg::with_name("credentials")
             .short("c")
             .long("cred")
             .help("Username and password for site if needed")
             .takes_value(true))
        .arg(Arg::with_name("first_url")
             .short("1")
             .long("first_url")
             .help("The first url you want to get the links from")
             .takes_value(true)
             .required(true))
        .arg(Arg::with_name("second_url")
             .short("2")
             .long("second_url")
             .help("The second url you want to get the links from")
             .takes_value(true)
             .required(true))
        .get_matches();

    
    let url_1 = matches.value_of("first_url").unwrap();
    let url_2 = matches.value_of("second_url").unwrap();
    let cred = matches.value_of("credentials").unwrap_or("");

    let mut links_1 = GetLinks::new(url_1.to_string(), cred.to_string());
    let dom_1 = links_1.gen_dom();
    links_1.find_links(dom_1.document);
    println!("{:#?}", links_1.get_links());
 
    let mut links_2 = GetLinks::new(url_2.to_string(), cred.to_string());
    let dom_2 = links_2.gen_dom();
    links_2.find_links(dom_2.document);
    println!("{:#?}", links_2.get_links());

    for link in links_1.get_links().iter() {
        println!("{} ~~~ {}", link, find_match(url_1, url_2, link, links_2.get_links()));
    }
}
