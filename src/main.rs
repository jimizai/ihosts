use clap::{App, Arg, SubCommand};
use ignore::WalkBuilder;
use prettytable::{Cell, Row, Table};
use std::io::{ErrorKind, Read};
use std::iter::Iterator;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{
    borrow::BorrowMut,
    fs::{self, File},
};

const WORKSPACE_DIR: &'static str = "/etc/ihosts";
const VERSION: &'static str = "0.0.1";
const AUTHOR: &'static str = "dyk <woshitiancai359@gmail.com>";
const HOST_FILE: &'static str = "/etc/hosts";
const IHOST_ANNOTATION: &'static str = "# hostname=";

struct BufLine<'a> {
    bufs: &'a [u8],
}

impl<'a> BufLine<'a> {
    pub fn new(bufs: &'a [u8]) -> Self {
        Self { bufs }
    }
}

impl<'a> Iterator for BufLine<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        for (index, buf) in self.bufs.iter().enumerate() {
            match buf {
                b'\n' => {
                    let bufs = &self.bufs[0..index];
                    self.bufs = &self.bufs[(index + 1)..self.bufs.len()];
                    return Some(bufs);
                }
                _ => (),
            }
        }

        if self.bufs.len() == 0 {
            None
        } else {
            Some(&self.bufs[0..self.bufs.len()])
        }
    }
}

fn read_file(path: &str) -> String {
    let buf = read_file_bufs(path);
    std::str::from_utf8(&buf).unwrap().to_string()
}

fn read_file_bufs(path: &str) -> Vec<u8> {
    let mut file = File::open(path).expect("Could not open hosts file");
    let mut buf = vec![];
    file.read_to_end(&mut buf).unwrap();
    buf
}

fn init_cteate_ihost_dir() {
    match fs::read_dir(WORKSPACE_DIR) {
        Err(why) if why.kind() == ErrorKind::NotFound => match fs::create_dir(WORKSPACE_DIR) {
            Err(why) => eprintln!("init error: {}", why),
            Ok(_) => {}
        },
        Err(why) => eprintln!("init error: {}", why),
        Ok(_) => (),
    }
}

fn read_base_dir() -> Vec<String> {
    let path = Path::new(WORKSPACE_DIR);
    let walker = WalkBuilder::new(path).build();
    walker
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().expect("no filetype").is_file())
        .map(|entry| String::from(entry.path().to_str().unwrap()))
        .map(|entry| match entry.split('/').last() {
            Some(s) => s.to_string(),
            None => String::new(),
        })
        .borrow_mut()
        .collect()
}

fn show_list() {
    let files = read_base_dir();
    let mut table = Table::new();
    table.add_row(Row::new(vec![Cell::new("NAME"), Cell::new("IS_USED")]));
    let used_hostnames = get_used_hostnames_from_hosts_file();

    for file in files {
        table.add_row(Row::new(vec![
            Cell::new(file.as_str()),
            Cell::new(used_hostnames.contains(&file).to_string().as_str()),
        ]));
    }
    table.printstd();
}

fn transform_path(file_name: &str) -> PathBuf {
    Path::new(WORKSPACE_DIR).join(Path::new(file_name))
}

fn get_host(file_name: &str) {
    let file = File::open(transform_path(file_name));
    let mut file = match file {
        Ok(f) => f,
        Err(_) => return eprintln!("not found"),
    };
    let mut buf = vec![];
    file.read_to_end(&mut buf).unwrap();
    println!("{}", std::str::from_utf8(&buf).unwrap());
}

fn set_host(file_name: &str) {
    let path = transform_path(file_name);
    match File::open(&path) {
        Ok(f) => f,
        Err(_) => File::create(&path).expect("create file error"),
    };
    Command::new("vim")
        .arg(&path)
        .status()
        .expect("Can't open editor");
}

fn get_used_hostnames_from_hosts_file() -> Vec<String> {
    let bufs = read_file_bufs(HOST_FILE);
    let lines = BufLine::new(&bufs);
    let mut v = vec![];
    let list = read_base_dir();
    for line in lines {
        let line = std::str::from_utf8(line).unwrap().trim_start().trim_end();
        if line.starts_with(IHOST_ANNOTATION) {
            let mut hostname = line.split(IHOST_ANNOTATION);
            let hostname = hostname.find(|s| s.len() != 0);
            if let Some(s) = hostname {
                let s = s.to_string();
                if list.contains(&s) {
                    v.push(s);
                }
            }
        }
    }
    v
}

fn write_hosts_file(hostnames: Vec<String>) {
    let mut bufs: Vec<u8> = Vec::new();
    for hostname in hostnames {
        let mut file = match File::open(Path::new(WORKSPACE_DIR).join(&hostname)) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let mut buf = vec![];
        file.read_to_end(&mut buf).unwrap();
        bufs.extend(IHOST_ANNOTATION.as_bytes());
        bufs.extend(hostname.as_bytes());
        bufs.extend(b"\n");
        bufs.extend(buf);
    }
    let data = std::str::from_utf8(&bufs).unwrap();
    fs::write(HOST_FILE, data).expect("write file error");
}

fn main() {
    init_cteate_ihost_dir();

    let matches = App::new("ihost")
        .version(VERSION)
        .author(AUTHOR)
        .about("a host manage util")
        .subcommand(SubCommand::with_name("list"))
        .subcommand(SubCommand::with_name("show"))
        .subcommand(
            SubCommand::with_name("get")
                .arg(Arg::with_name("hostname").takes_value(true).required(true)),
        )
        .subcommand(
            SubCommand::with_name("use")
                .arg(Arg::with_name("hostname").takes_value(true).required(true)),
        )
        .subcommand(
            SubCommand::with_name("un")
                .alias("unuse")
                .arg(Arg::with_name("hostname").takes_value(true).required(true)),
        )
        .subcommand(
            SubCommand::with_name("set")
                .arg(Arg::with_name("hostname").takes_value(true).required(true)),
        )
        .subcommand(
            SubCommand::with_name("rm")
                .alias("remove")
                .arg(Arg::with_name("hostname").takes_value(true).required(true)),
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("show") {
        let result = read_file(HOST_FILE);
        println!("{}", result);
    } else if let Some(_) = matches.subcommand_matches("list") {
        show_list();
    } else if let Some(m) = matches.subcommand_matches("get") {
        get_host(m.args.get("hostname").unwrap().vals[0].to_str().unwrap());
    } else if let Some(m) = matches.subcommand_matches("set") {
        let hostnames = get_used_hostnames_from_hosts_file();
        let hostname = m.args.get("hostname").unwrap().vals[0].to_str().unwrap();
        set_host(&hostname);
        if hostnames.contains(&hostname.to_string()) {
            write_hosts_file(hostnames);
        }
        show_list();
    } else if let Some(m) = matches.subcommand_matches("use") {
        let mut hostnames = get_used_hostnames_from_hosts_file();
        let hostname = m.args.get("hostname").unwrap().vals[0]
            .to_str()
            .unwrap()
            .to_string();
        if !hostnames.contains(&hostname) {
            hostnames.push(hostname)
        }
        write_hosts_file(hostnames);
        show_list();
    } else if let Some(m) = matches.subcommand_matches("un") {
        let hostnames = get_used_hostnames_from_hosts_file();
        let hostname = m.args.get("hostname").unwrap().vals[0].to_str().unwrap();
        let v = hostnames
            .into_iter()
            .filter(|s| s != hostname)
            .collect::<Vec<String>>();
        write_hosts_file(v);
        show_list();
    } else if let Some(m) = matches.subcommand_matches("rm") {
        let hostnames = get_used_hostnames_from_hosts_file();
        let hostname = m.args.get("hostname").unwrap().vals[0].to_str().unwrap();
        if hostnames.contains(&hostname.to_string()) {
            let v = hostnames
                .into_iter()
                .filter(|s| s != hostname)
                .collect::<Vec<String>>();
            write_hosts_file(v);
        }
        match fs::remove_file(transform_path(hostname)) {
            Ok(_) => {}
            Err(_) => eprintln!("remove file error"),
        }
        show_list();
    }
}
