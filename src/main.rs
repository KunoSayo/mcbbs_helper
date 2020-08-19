//https://doc.rust-lang.org/book/


use std::collections::LinkedList;
use std::fs::File;
use std::io::{Read, stdin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use chrono::{DateTime, Local};
use regex::Regex;
use tokio::time::Duration;

const VANILLA: &str = "https://www.mcbbs.net/forum-qanda-1.html?mobile=no";
const MU: &str = "https://www.mcbbs.net/forum-multiqanda-1.html?mobile=no";
const MOD: &str = "https://www.mcbbs.net/forum-modqanda-1.html?mobile=no";
const AROUND: &str = "https://www.mcbbs.net/forum-etcqanda-1.html?mobile=no";
const VOID: &str = "https://www.mcbbs.net/forum-peqanda-1.html?mobile=no";

static OPEN: AtomicBool = AtomicBool::new(false);

async fn get(url: &str) {
    let pattern: Regex = Regex::new(".*(?P<url>thread-[0-9]+-[0-9]+-[0-9]+\\.html).*class=\"s xst\">(?P<title>.*)\n*</a>\n*").unwrap();

    let mut cookie_file = File::open("cookie.cookie").unwrap();
    let mut cookie = String::new();
    cookie_file.read_to_string(&mut cookie).unwrap();
    let mut list = LinkedList::new();
    loop {
        match reqwest::Client::new().get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/75.0.3770.100 Safari/537.36")
            .header("Cookie", &cookie)
            .timeout(Duration::from_secs(5)).send().await {
            Ok(resp) => {
                let lines: Vec<String> = resp.text().await.map(|c| c.lines().map(|l| l.to_string()).collect()).unwrap();
                let mut i = 0;
                let mut found = false;
                while i < lines.len() {
                    let line = &lines[i];

                    let matcher = pattern.captures(&*line);
                    if let Some(matcher) = matcher {
                        for j in 0..6 {
                            if lines[i + j].contains("金粒") {
                                if !list.contains(line) {
                                    if !found {
                                        println!();
                                        found = true;
                                    }
                                    list.push_back(line.clone());
                                    while list.len() > 70 {
                                        list.pop_front();
                                    }
                                    if OPEN.load(Ordering::Relaxed) {
                                        if let Err(e) = webbrowser::open(&*format!("https://www.mcbbs.net/{}", &matcher["url"])) {
                                            eprintln!("open failed {}", e);
                                        }
                                    }
                                    let idx = lines[i + j].find(r#""xw1">"#).unwrap_or(0);
                                    println!("{} \"https://www.mcbbs.net/{}\" {} {}", DateTime::<Local>::from(SystemTime::now()).time().format("%H:%M:%S")
                                        .to_string(), &matcher["url"], &matcher["title"], &lines[i + j][idx + 5..]);
                                }
                                break;
                            }
                        }
                    }


                    i += 1;
                }
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        }
        tokio::time::delay_for(Duration::from_secs_f32(5.0)).await;
    }
}

async fn water() {
    let pattern: Regex = Regex::new(r#".*viewthread.*tid=(?P<tid>\d+).*"s xst">(?P<title>.*)</a>.*"#).unwrap();
    let mut list = LinkedList::new();
    loop {
        match reqwest::Client::new().get("https://www.mcbbs.net/forum.php?mod=forumdisplay&fid=52&filter=author&orderby=dateline&mobile=no")
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/75.0.3770.100 Safari/537.36")
            .timeout(Duration::from_secs(5)).send().await {
            Ok(resp) => {
                if let Ok(content) = resp.text().await {
                    let lines: Vec<&str> = content.lines().collect();
                    for line in lines {
                        let matcher = pattern.captures(&*line);
                        if let Some(matcher) = matcher {
                            let tid = &matcher["tid"];
                            if tid.parse().unwrap_or(9000000) > 1000000 {
                                if !list.contains(&tid.to_string()) {
                                    list.push_back(tid.to_string());
                                    while list.len() > 70 {
                                        list.pop_front();
                                    }
                                    if OPEN.load(Ordering::Relaxed) {
                                        if let Err(e) = webbrowser::open(&*format!("https://www.mcbbs.net/thread-{}-1-1.html", &matcher["tid"])) {
                                            eprintln!("open failed {}", e);
                                        }
                                    }
                                    println!("{} https://www.mcbbs.net/thread-{}-1-1.html {}", DateTime::<Local>::from(SystemTime::now()).time().format("%H:%M:%S")
                                        .to_string(), &matcher["tid"], &matcher["title"]);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("get water failed: {}", e)
            }
        }
        tokio::time::delay_for(Duration::from_secs_f32(5.0)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("1 2 3 4 5 爬 单 多 模 周 虚");
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).expect("read input failed");
        let input = input.trim();
        match input {
            "1" => {
                println!("getting VANILLA question");
                tokio::spawn(async {
                    get(VANILLA).await;
                    println!("爬vanilla结束")
                });
            }
            "2" => {
                println!("getting MU question");
                tokio::spawn(async {
                    get(MU).await;
                    println!("爬MU结束")
                });
            }
            "3" => {
                println!("getting MOD question");
                tokio::spawn(async {
                    get(MOD).await;
                    println!("爬MOD结束")
                });
            }
            "4" => {
                println!("getting AROUND question");
                tokio::spawn(async {
                    get(AROUND).await;
                    println!("爬AROUND结束")
                });
            }
            "5" => {
                println!("getting VOID question");
                tokio::spawn(async {
                    get(VOID).await;
                    println!("爬VOID结束")
                });
            }
            "all" => {
                println!("getting VANILLA question");
                tokio::spawn(async {
                    get(VANILLA).await;
                    println!("爬vanilla结束")
                });

                println!("getting MU question");
                tokio::spawn(async {
                    get(MU).await;
                    println!("爬MU结束")
                });
                println!("getting MOD question");
                tokio::spawn(async {
                    get(MOD).await;
                    println!("爬MOD结束")
                });
                println!("getting AROUND question");
                tokio::spawn(async {
                    get(AROUND).await;
                    println!("爬AROUND结束")
                });
                println!("getting VOID question");
                tokio::spawn(async {
                    get(VOID).await;
                    println!("爬VOID结束")
                });
            }
            "water" => {
                println!("getting water");
                tokio::spawn(async {
                    water().await;
                    println!("爬water结束")
                });
            }
            "on" => {
                OPEN.swap(true, Ordering::Relaxed);
                println!("enabled open.");
            }
            "off" => {
                OPEN.swap(false, Ordering::Relaxed);
                println!("disabled open.");
            }
            "stop" => break,
            _ => {}
        }
    }
    Ok(())
}
