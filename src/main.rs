///! A program that crawls a webpage and grabs all the links.
///! Then, through some machine learning, it will translate those "old"
///! links and convert them to the new links

extern crate curl;
extern crate html5ever;
extern crate clap;
extern crate regex;

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

use regex::Regex;

fn domain_permutations(url: &str) -> Vec<String> {
    let com = url.find(".com");
    if url.find(".") != com {
        return vec![url.to_string()];
    }
    if url.find("www.") == Some(7) {
        vec![url.to_string(), url.replace("www.", "").to_string()]
    }
    else {
        let url2 = format!("{}www.{}", &url[0..7], &url[7..]);
        vec![url2, url.to_string()]
    }
}

fn is_internal_link(link: &str, domain_permutations: &Vec<String>) -> bool {
    if link.is_empty() {
        return false;
    }

    let domain_match = Regex::new(r"^https?://").unwrap();

    if !domain_match.is_match(link) {
        let re = Regex::new(r"^/|^[a-zA-Z0-9]").unwrap();
        if re.is_match(link) && !Regex::new("^javascript").unwrap().is_match(link) {
            println!("found match thing: {}", link);
            return true;
        }

    } else {
        println!("has a link");
        for domain in domain_permutations.iter() {
            if link.contains(domain) {
                if &link[0.. domain.len()] == domain {
                    return true;
                }
            }
        }
    }
    false
}
struct GetLinks {
    links: Vec<StrTendril>,
    urls: Vec<String>,
    pwd: String
}

impl GetLinks {
    pub fn new(url: String, pwd: String) -> GetLinks {
        GetLinks {
            links: Vec::new(),
            urls: domain_permutations(&url),
            pwd: pwd
        }
    }

    pub fn gen_dom(&self) -> RcDom {
        let resp = http::handle()
            .userpwd(&self.pwd[..])
            .get(&self.urls[0][..])
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
                    let mut href_attr = attrs.iter().filter(|&x| x.name.local.as_slice() == "href").collect::<Vec<_>>();
                    if !href_attr.is_empty() {
                        let link = href_attr[0].value.clone();
                        println!("{} {:#?}", name.local, link);
                        if !link.is_empty()  {
                            if is_internal_link(&link, &self.urls) {
                                self.links.push(link.clone());
                            }
                        }
                    }
                }
            },
            _ => { }
        }

        self.links.sort();
        self.links.dedup();

        for child in node.children.iter() {
            self.find_links(child.clone());
        }

    }

    pub fn get_links(&self) -> Vec<StrTendril> { return self.links.clone(); }
    pub fn get_url(&self) -> Vec<String> { return self.urls.clone(); }
    pub fn get_pwd(&self) -> String { return self.pwd.clone(); }
}

fn avg_key_match(i: &str, j: &str) -> f64 {
    let re = match Regex::new(r"[a-zA-Z0-9]+") {
        Ok(r) => r,
        Err(e) => panic!("Unable to create regex, {}", e)
    };

    // get the keywords in the url based on the regex
    let i_keywords: Vec<&str> = re.find_iter(i).map(|(t, f)| &i[t..f]).collect();
    let j_keywords: Vec<&str> = re.find_iter(j).map(|(t, f)| &j[t..f]).collect();

    if i_keywords.is_empty() || j_keywords.is_empty() {
        return 0f64;
    }

    // vars for future use
    let mut cnt: f64 = 0f64;
    let i_len: f64 = i_keywords.len() as f64;
    let j_len: f64 = j_keywords.len() as f64;

    let mut matches: Vec<&str> = Vec::new();

    for i_key in i_keywords.iter() {
        for j_key in j_keywords.iter() {
            if j_key == i_key {
                cnt += 1f64;
                matches.push(j_key);
            }
        }
    }

    /*
    println!("{:?}", matches);
    println!("({} / {}) * ({} / {}): {}", cnt, i_len, cnt, j_len, (cnt / i_len) * (cnt / j_len));
    */
    (cnt / i_len) * (cnt / j_len)
}

fn get_route<'a>(link: &'a str, domain_permutations: &Vec<String>) -> Option<&'a str> {
    let mut end = link.len();

    let re = Regex::new(r"html|htm|php|asp").unwrap();
    if let Some((to, _)) = re.find(link) {
        end = to;
    }
    for domain in domain_permutations.iter() {
        if link.contains(domain) {
            if &link[0.. domain.len()] == &domain[..] {
                return Some(&link[domain.len() .. end]);
            }
        }
    }

    if Regex::new("^/|^[a-zA-Z0-9]").unwrap().is_match(link) {
        return Some(&link[.. end]);
    }
    None
}

fn find_match(u1: &str, u2: &str, url1: &StrTendril, links: Vec<StrTendril>) -> (f64, StrTendril) {
    let u: &str = match get_route(&url1, &domain_permutations(u1)) {
        Some(route) => route,
        None => panic!("Parsing route error: {}", url1)
    };

    let mut match_tendril = links[0].clone();
    let u2_permutations = domain_permutations(&u2);
    let l: &str = match get_route(&links[0], &u2_permutations) {
        Some(route) => route,
        None => panic!("Parsing route error: {}", links[0])
    };

    let mut edits = avg_key_match(u, l);

    for link in links.iter().skip(1) {
        let l: &str = match get_route(&link, &u2_permutations) {
            Some(route) => route,
            None => panic!("Parsing route error: {}", link)
        };

        let e = avg_key_match(u, l);
        if e > edits {
            edits = e;
            match_tendril = (*link).clone().to_tendril();
        }
    }
    (edits, match_tendril)
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
        println!("best match: {} ~~ {:#?}", link, find_match(url_1, url_2, link, links_2.get_links()));
    }
}
