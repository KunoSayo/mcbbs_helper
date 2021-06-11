//https://doc.rust-lang.org/book/

use std::collections::{HashMap, LinkedList, VecDeque};
use std::fs::File;
use std::io::{Read, stdin};
use std::sync::{Arc, Weak};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::SystemTime;

use chrono::{DateTime, Local};
use regex::Regex;
use tokio::sync::{Mutex, RwLock};
use tokio::time::Duration;

#[cfg(feature = "admin")]
mod admin;

const VANILLA: &str = "https://www.mcbbs.net/forum-qanda-1.html?mobile=no";
const MU: &str = "https://www.mcbbs.net/forum-multiqanda-1.html?mobile=no";
const MOD: &str = "https://www.mcbbs.net/forum-modqanda-1.html?mobile=no";
const AROUND: &str = "https://www.mcbbs.net/forum-etcqanda-1.html?mobile=no";
const VOID: &str = "https://www.mcbbs.net/forum-peqanda-1.html?mobile=no";

static OPEN: AtomicBool = AtomicBool::new(false);


/// What THE ** CODE IT IS? ARC ARC ARC
struct McbbsThreadData {
    mcbbs: Weak<McbbsData>,
    tid: u32,
    cd: u64,
    running: AtomicBool,
    last_replay: AtomicU64,
}

impl McbbsThreadData {
    async fn run(&self) {
        let pattern: Regex = Regex::new(r##".*<span class="xi1">(?P<num>\d*)</span>.*"##).unwrap();

        while self.running.load(Ordering::SeqCst) {
            if let Some(mcbbs) = self.mcbbs.upgrade() {
                match mcbbs.get_content(&format!("https://www.mcbbs.net/thread-{}-1-1.html", self.tid)).await {
                    Ok(lines) => {
                        let mut i = 0;
                        'while_loop: while i < lines.len() {
                            if lines[i].contains(r#"<span class="xg1">回复"#) {
                                for j in 0..5 {
                                    if let Some(matcher) = pattern.captures(&lines[i + j]) {
                                        let count = matcher["num"].parse().unwrap();
                                        let last = self.last_replay.load(Ordering::Acquire);
                                        if count > last {
                                            println!(r#"{} | https://www.mcbbs.net/thread-{}-1-1.html 的回复增加了[{}->{}]"#,
                                                     DateTime::<Local>::from(SystemTime::now()).format("%H:%M:%S").to_string(),
                                                     self.tid, last, count);
                                            self.last_replay.store(count, Ordering::Release);
                                        } else if count < last {
                                            println!(r#"{} | https://www.mcbbs.net/thread-{}-1-1.html 的回复减少了[{}->{}]"#,
                                                     DateTime::<Local>::from(SystemTime::now()).format("%H:%M:%S").to_string(),
                                                     self.tid, last, count);
                                            self.last_replay.store(count, Ordering::Release);
                                        }
                                        break 'while_loop;
                                    }
                                }
                            }

                            i += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                        tokio::time::delay_for(Duration::from_millis(self.cd + 3000)).await;
                    }
                }
                tokio::time::delay_for(Duration::from_millis(self.cd)).await;
            } else {
                println!("mcbbs destroyed.");
                break;
            }
        }
    }

    fn new(mcbbs: Arc<McbbsData>, tid: u32, cd: u64) -> Self {
        let week = Arc::downgrade(&mcbbs);
        Self {
            mcbbs: week,
            tid,
            cd,
            running: AtomicBool::new(true),
            last_replay: Default::default(),
        }
    }
}


/// What THE ** CODE IT IS? MUTEX MUTEX MUTEX ARC ARC ARC
pub struct McbbsData {
    questions: RwLock<LinkedList<String>>,
    question_cd: AtomicU64,
    water_cd: AtomicU64,
    #[cfg(feature = "admin")]
    admin_cd: AtomicU64,
    request_lock: Mutex<()>,
    listening_threads: Mutex<HashMap<u32, Arc<McbbsThreadData>>>,
    cookie: String,
}

impl Default for McbbsData {
    fn default() -> Self {
        let mut cookie = String::new();
        match File::open("cookie.cookie") {
            Ok(mut cookie_file) => {
                if let Err(e) = cookie_file.read_to_string(&mut cookie) {
                    println!("read cookie file failed. {}", e);
                }
            }
            Err(e) => {
                eprintln!("read cookie file failed. {}", e);
            }
        }
        Self {
            questions: Default::default(),
            question_cd: AtomicU64::new(5_000),
            water_cd: AtomicU64::new(15_000),
            #[cfg(feature = "admin")]
            admin_cd: AtomicU64::new(15_000),
            request_lock: Default::default(),
            listening_threads: Default::default(),
            cookie,
        }
    }
}

impl McbbsData {
    async fn get_content(&self, url: &str) -> Result<Vec<String>, reqwest::Error> {
        let _lock = self.request_lock.lock().await;
        let response = reqwest::Client::new().get(url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/75.0.3770.100 Safari/537.36")
            .header("Cookie", &self.cookie)
            .timeout(Duration::from_secs(10)).send().await?;
        match response.error_for_status() {
            Ok(r) => {
                Ok(r.text().await?.lines().map(&str::to_string).collect())
            }
            Err(e) => {
                tokio::time::delay_for(Duration::from_secs_f32(5.0)).await;
                Err(e)
            }
        }
    }

    async fn run_get_task(&self, url: &str) {
        let pattern: Regex = Regex::new(".*(?P<url>thread-[0-9]+-[0-9]+-[0-9]+\\.html).*class=\"s xst\">(?P<title>.*)\n*</a>\n*").unwrap();

        loop {
            match self.get_content(url).await {
                Ok(lines) => {
                    let mut list = self.questions.read().await;
                    let mut i = 0;
                    let mut found = false;
                    let mut cache = VecDeque::new();
                    while i < lines.len() {
                        let line = &lines[i];

                        let matcher = pattern.captures(&line);
                        if let Some(matcher) = matcher {
                            for j in 0..6 {
                                if lines[i + j].contains("金粒") {
                                    if !list.contains(line) {
                                        if !found {
                                            println!();
                                            found = true;
                                        }
                                        cache.push_back(line);
                                        if OPEN.load(Ordering::Acquire) {
                                            if let Err(e) = webbrowser::open(&format!("https://www.mcbbs.net/{}", &matcher["url"])) {
                                                eprintln!("open failed {}", e);
                                            }
                                        }
                                        let idx = lines[i + j].find(r#""xw1">"#).unwrap_or(0);
                                        println!("{} \"https://www.mcbbs.net/{}\" {} {}", DateTime::<Local>::from(SystemTime::now()).format("%H:%M:%S")
                                            .to_string(), &matcher["url"], &matcher["title"], &lines[i + j][idx + 5..]);
                                        list = self.questions.read().await;
                                    }
                                    break;
                                }
                            }
                        }
                        i += 1;
                    }
                    if cache.len() > 0 {
                        std::mem::drop(list);
                        let mut list = self.questions.write().await;
                        while let Some(v) = cache.pop_front() {
                            list.push_back(v.clone());
                        }
                        while list.len() > 249 {
                            list.pop_front();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", e);
                    tokio::time::delay_for(Duration::from_millis(self.question_cd.load(Ordering::Relaxed) + 3000)).await;
                }
            }
            tokio::time::delay_for(Duration::from_millis(self.question_cd.load(Ordering::Relaxed))).await;
        }
    }


    async fn water(&self) {
        let mut vec = VecDeque::new();
        let pattern: Regex = Regex::new(r#".*viewthread.*tid=(?P<tid>\d+).*"s xst">(?P<title>.*)</a>.*"#).unwrap();
        loop {
            match self.get_content("https://www.mcbbs.net/forum.php?mod=forumdisplay&fid=52&filter=author&orderby=dateline&mobile=no").await {
                Ok(lines) => {
                    for line in lines {
                        let matcher = pattern.captures(&line);
                        if let Some(matcher) = matcher {
                            let tid = &matcher["tid"];
                            if tid.parse().unwrap_or(9000000) > 1000000 {
                                if let Some(idx) = vec.iter().position(|s| s == tid) {
                                    let got = vec.swap_remove_back(idx);
                                    vec.push_back(got.unwrap());
                                } else {
                                    vec.push_back(tid.to_string());
                                    while vec.len() > 70 {
                                        vec.pop_front();
                                    }
                                    if OPEN.load(Ordering::Relaxed) {
                                        if let Err(e) = webbrowser::open(&format!("https://www.mcbbs.net/thread-{}-1-1.html", &matcher["tid"])) {
                                            eprintln!("open failed {}", e);
                                        }
                                    }
                                    println!("{} https://www.mcbbs.net/thread-{}-1-1.html {}", DateTime::<Local>::from(SystemTime::now()).format("%H:%M:%S")
                                        .to_string(), &matcher["tid"], &matcher["title"]);
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("get water failed: {}", e);
                    tokio::time::delay_for(Duration::from_millis(self.water_cd.load(Ordering::Relaxed) + 10000)).await;
                }
            }
            tokio::time::delay_for(Duration::from_millis(self.water_cd.load(Ordering::Relaxed))).await;
        }
    }

    async fn view_water(&self) {
        let pattern: Regex = Regex::new(r#".*thread-(?P<tid>\d+)-.*"s xst">(?P<title>.*)</a>.*"#).unwrap();
        let span_pattern: Regex = Regex::new(r#".*<span.*?>(?P<content>.*)</span>.*"#).unwrap();

        match self.get_content("https://www.mcbbs.net/forum-chat-1.html?mobile=no").await {
            Ok(lines) => {
                for (idx, line) in lines.iter().enumerate() {
                    let matcher = pattern.captures(line);
                    if let Some(matcher) = matcher {
                        for i in idx..lines.len() {
                            let line = &lines[i];
                            if !line.contains("tps") {
                                if let Some(span_matcher) = span_pattern.captures(line) {
                                    println!("{} https://www.mcbbs.net/thread-{}-1-1.html {}",
                                             &span_matcher["content"].replace("&nbsp;", " ")
                                                 .replace("</span>", ""),
                                             &matcher["tid"], &matcher["title"]);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("view water failed: {}", e);
                tokio::time::delay_for(Duration::from_millis(self.water_cd.load(Ordering::Relaxed) + 10000)).await;
            }
        }
    }

    async fn get_vote_info(&self, url: &str) {
        let url = {
            let lines = match self.get_content(url).await {
                Ok(line) => line,
                Err(e) => {
                    eprintln!("get vote info from thread error: {}", e);
                    return;
                }
            };
            let pattern = Regex::new(r#".*href="(?P<url>.*?)".*查看投票参与人.*"#).unwrap();
            let mut url = String::from("https://www.mcbbs.net/");
            let mut found = false;
            for line in lines {
                if let Some(matcher) = pattern.captures(&line) {
                    url += &matcher["url"].replace("&amp;", "&");
                    found = true;
                    break;
                }
            };
            if !found {
                eprintln!("cannot found vote request url.");
                return;
            }
            url
        };
        let lines = match self.get_content(&url).await {
            Ok(line) => line,
            Err(e) => {
                eprintln!("get vote info from first request error: {}", e);
                return;
            }
        };
        let pattern = Regex::new(r#".*option value="(?P<value>\d+)".*>(?P<text>.*)</option>.*"#).unwrap();
        let voter_pattern = Regex::new(r#"href="(?P<url>.*)">(?P<name>.*)</a>.*"#).unwrap();
        let mut values = Vec::with_capacity(30);
        let mut results = Vec::with_capacity(30);
        results.push(vec![]);
        for i in 0..lines.len() {
            let line = &lines[i];
            if line.contains(r#"select class="ps""#) {
                for j in i..i + 35 {
                    if let Some(matcher) = pattern.captures(&lines[j]) {
                        values.push((matcher["value"].parse::<u32>().unwrap(), matcher["text"].to_string()));
                    }
                }
            } else if line.contains("voter") && line.contains("ul") {
                for i in i..lines.len() {
                    let line = &lines[i];
                    if line.contains("</ul>") {
                        break;
                    }
                    if let Some(matcher) = voter_pattern.captures(line) {
                        results[0].push(matcher["name"].to_string());
                    }
                }
                break;
            }
        }

        for x in &values[1..] {
            results.push(vec![]);
            tokio::time::delay_for(Duration::from_micros(169961)).await;

            let url = format!("{}&polloptionid={}", url, x.0);
            let lines = match self.get_content(&url).await {
                Ok(lines) => lines,
                Err(e) => {
                    eprintln!("get vote info from {} error: {}", x.0, e);
                    continue;
                }
            };
            for i in 0..lines.len() {
                let line = &lines[i];
                if line.contains("voter") && line.contains("ul") {
                    for i in i..lines.len() {
                        let line = &lines[i];
                        if line.contains("</ul>") {
                            break;
                        }
                        if let Some(matcher) = voter_pattern.captures(line) {
                            results.last_mut().unwrap().push(matcher["name"].to_string());
                        }
                    }
                    break;
                }
            }
        }
        for i in 0..results.len() {
            println!("[{:02}] #{:02} {}: {}",
                     i + 1,
                     results[i].len(),
                     values[i].1,
                     results[i].join(", "));
        }
    }

    async fn info(&self) {
        let w = self.water_cd.load(Ordering::Relaxed);
        let q = self.question_cd.load(Ordering::Relaxed);
        println!("water_cd_ms: {}, question_cd_ms: {}", w, q);
        #[cfg(feature = "admin")]
            {
                let a = self.admin_cd.load(Ordering::Relaxed);
                println!("admin_cd_ms: {}", a);
            }
        let map = self.listening_threads.lock().await;
        for (k, d) in map.iter() {
            println!("listening tid {} interval {}ms with replay_count: {}", *k, d.cd, d.last_replay.load(Ordering::Relaxed));
        }
        println!("open enabled: {}", OPEN.load(Ordering::Acquire));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mcbbs = Arc::new(McbbsData::default());
    println!("1 2 3 4 5 爬 单 多 模 周 虚");
    loop {
        let mut input = String::new();
        stdin().read_line(&mut input).expect("read input failed");
        let raw_input = input.trim();
        let input: Vec<&str> = raw_input.splitn(2, " ").collect();
        match input[0] {
            "1" => {
                let mcbbs = mcbbs.clone();
                println!("getting VANILLA question");
                tokio::spawn(async move {
                    mcbbs.run_get_task(VANILLA).await;
                    println!("爬vanilla结束")
                });
            }
            "2" => {
                let mcbbs = mcbbs.clone();
                println!("getting MU question");
                tokio::spawn(async move {
                    mcbbs.run_get_task(MU).await;
                    println!("爬MU结束")
                });
            }
            "3" => {
                let mcbbs = mcbbs.clone();
                println!("getting MOD question");
                tokio::spawn(async move {
                    mcbbs.run_get_task(MOD).await;
                    println!("爬MOD结束")
                });
            }
            "4" => {
                let mcbbs = mcbbs.clone();
                println!("getting AROUND question");
                tokio::spawn(async move {
                    mcbbs.run_get_task(AROUND).await;
                    println!("爬AROUND结束")
                });
            }
            "5" => {
                let mcbbs = mcbbs.clone();
                println!("getting VOID question");
                tokio::spawn(async move {
                    mcbbs.run_get_task(VOID).await;
                    println!("爬VOID结束")
                });
            }
            "all" => {
                let mcbbs_arc = mcbbs.clone();
                println!("getting VANILLA question");
                tokio::spawn(async move {
                    mcbbs_arc.run_get_task(VANILLA).await;
                    println!("爬vanilla结束")
                });
                let mcbbs_arc = mcbbs.clone();
                println!("getting MU question");
                tokio::spawn(async move {
                    mcbbs_arc.run_get_task(MU).await;
                    println!("爬MU结束")
                });
                let mcbbs_arc = mcbbs.clone();
                println!("getting MOD question");
                tokio::spawn(async move {
                    mcbbs_arc.run_get_task(MOD).await;
                    println!("爬MOD结束")
                });
                let mcbbs_arc = mcbbs.clone();
                println!("getting AROUND question");
                tokio::spawn(async move {
                    mcbbs_arc.run_get_task(AROUND).await;
                    println!("爬AROUND结束")
                });
                let mcbbs_arc = mcbbs.clone();
                println!("getting VOID question");
                tokio::spawn(async move {
                    mcbbs_arc.run_get_task(VOID).await;
                    println!("爬VOID结束")
                });
            }
            "water" => {
                let mcbbs = mcbbs.clone();
                println!("getting water");
                tokio::spawn(async move {
                    mcbbs.water().await;
                    println!("爬water结束")
                });
            }
            "on" => {
                OPEN.store(true, Ordering::Release);
                println!("enabled open.");
            }
            "off" => {
                OPEN.store(false, Ordering::Release);
                println!("disabled open.");
            }
            "mod" => {
                if let Err(e) = webbrowser::open("https://www.mcbbs.net/forum.php?mod=forumdisplay&fid=45&filter=sortid&sortid=1") {
                    eprintln!("{}", e);
                }
            }
            "live" => {
                if let Err(e) = webbrowser::open("https://link.bilibili.com/p/center/index#/my-room/start-live") {
                    eprintln!("{}", e);
                }
            }
            "code" => {
                println!("[font=Consolas][color=#A9B7C6][table=98%,Black]
[tr=#2F2F2F][td][p=15, 0, left]
[/p][/td][/tr][/table][/color][/font]")
            }
            "info" => {
                println!("---begin info---");

                mcbbs.info().await;
                println!("----end info----");
            }
            "listen" => {
                let args = input[1].splitn(2, " ").collect::<Vec<&str>>();
                if args.len() < 2 {
                    println!("listen <tid> <cd>")
                } else {
                    if let (Ok(tid), Ok(cd)) = (args[0].parse::<u32>(), args[1].parse()) {
                        if cd < 1000 {
                            let mcbbs = mcbbs.clone();
                            tokio::spawn(async move {
                                let mut map = mcbbs.listening_threads.lock().await;
                                if let Some(d) = map.remove(&tid) {
                                    d.running.store(false, Ordering::SeqCst);
                                    println!("stop listening replay for {}", tid);
                                } else {
                                    println!("no task for listening replay for {}", tid);
                                }
                            });
                        } else {
                            let mcbbs = mcbbs.clone();
                            tokio::spawn(async move {
                                let mut map = mcbbs.listening_threads.lock().await;
                                let data = Arc::new(McbbsThreadData::new(mcbbs.clone(), tid, cd));
                                if let Some(old) = map.insert(tid, data.clone()) {
                                    old.running.store(false, Ordering::SeqCst);
                                }
                                println!("start listening replay for {}", tid);
                                std::mem::drop(map);
                                data.run().await;
                            });
                        }
                    } else {
                        println!("parse number failed.");
                    }
                }
            }
            "cd" => {
                let input = if input.len() == 1 { vec![] } else { input[1].splitn(2, " ").collect::<Vec<&str>>() };
                if input.len() <= 1 {
                    println!("Unknown source/cd time.");
                } else if let Ok(cd) = input[1].parse() {
                    if cd < 1000 {
                        println!("cd is too short for {}ms", cd);
                    } else {
                        match input[0] {
                            "water" => mcbbs.water_cd.store(cd, Ordering::Relaxed),
                            "question" => mcbbs.question_cd.store(cd, Ordering::Relaxed),
                            #[cfg(feature = "admin")]
                            "admin" => mcbbs.admin_cd.store(cd, Ordering::Relaxed),
                            _ => {
                                println!("Unknown cd source: {}", input[0])
                            }
                        }
                    }
                } else {
                    println!("cd is number.");
                }
            }
            "vw" => {
                let mcbbs = mcbbs.clone();
                tokio::spawn(async move {
                    println!("---vw");
                    mcbbs.view_water().await;
                    println!("---vw");
                });
            }
            "vote" => {
                if input.len() == 2 {
                    println!("getting vote info from {}.", input[1]);
                    mcbbs.get_vote_info(input[1]).await;
                    println!("end getting vote info.");
                }
            }
            "view" => {
                println!(r#"https://www.mcbbs.net/home.php?mod=space&uid={}&do=index"#
                         , input.get(1).unwrap_or(&"{UID}"));
            }
            "name" => {
                if let Err(e) = webbrowser::open(&format!("https://www.mcbbs.net/home.php?mod=space&username={}",
                                                          input.get(1).unwrap_or(&""))) {
                    eprintln!("open failed {}", e);
                }
            }
            "stop" => break,
            _ => {
                #[cfg(feature = "admin")]
                    admin::process(mcbbs.clone(), raw_input);
            }
        }
    }

    Ok(())
}
