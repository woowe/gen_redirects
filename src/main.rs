///! A program that crawls a webpage and grabs all the links.
///! Then, through some machine learning, it will translate those "old"
///! links and convert them to the new links

use std::env;

extern crate curl;
extern crate html5ever;

#[macro_use]
extern crate string_cache;
extern crate tendril;

use curl::http;

use html5ever::{parse, one_input};
use html5ever::rcdom::{Element, RcDom, Handle};

use tendril::{SliceExt};

fn walk(handle: Handle, url: &str) {
    let node = handle.borrow();

    match node.node {
        Element(ref name, _, ref attrs) => {
            assert!(name.ns == ns!(html));
            if name.local.as_slice() == "a" {
                for attr in attrs.iter() {
                    assert!(attr.name.ns == ns!(""));
                    if attr.name.local.as_slice() == "href" {
                        let href_val = attr.value.clone();
                        if !href_val.is_empty()  {
                            let first = &href_val[0..1];
                            if first == "/" || (&href_val).contains(url) {
                                println!("<a {}=\"{}\" >", attr.name.local, attr.value);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }

    for child in node.children.iter() {
        walk(child.clone(), url);
    }
}

fn get_dom(url: &str) -> RcDom {
    let resp = http::handle()
        .get(url)
        .exec().unwrap();


    let input = resp.get_body().to_tendril();
    let input = input.try_reinterpret().unwrap();

    parse(one_input(input), Default::default())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).map(|x| x).collect();
    if args.is_empty() {
        println!("Please provide a url! (i.e http://www.google.com)");
        return;
    }
    println!("Program is starting...");

    let dom = get_dom(&args[0][..]);

    walk(dom.document, &args[0][..]);

    if !dom.errors.is_empty() {
        println!("\nParse Errors:");
        for err in dom.errors.into_iter() {
            println!("    {}", err);
        }
    }
}
