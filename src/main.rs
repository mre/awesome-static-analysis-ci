// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

extern crate afterparty;
extern crate hyper;

use afterparty::{Delivery, Hub};

use hyper::Server;

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate error_chain;
extern crate regex;
extern crate reqwest;

use afterparty::Event;
use std::collections::HashMap;
use regex::Regex;
use std::io::Read;
use std::env;

lazy_static! {
    static ref TOOL_REGEX: Regex = Regex::new(r"\*\s\[(.*)\]\((http[s]?://.*)\)\s(:copyright:\s)?\-\s(.*)").unwrap();
    static ref SUBSECTION_HEADLINE_REGEX: Regex = Regex::new(r"[A-Za-z\s]*").unwrap();
}

error_chain!{
    foreign_links {
        IoError(std::io::Error);
        ReqwestError(reqwest::Error);
        HyperError(hyper::Error);
        EnvironmentError(std::env::VarError);
    }

    errors {
        EmptySection {
            description("Empty section")
            display("A tool section may not be empty")
        }

        BrokenLink(url: String) {
            description("The URL does not appear to be correct")
            display("Invalid tool URL: {}", url)
        }

        InvalidTool(raw: String) {
            description("Invalid tool")
            display("Invalid tool: {}", raw)
        }
    }
}

enum Status {
    Success,
    Pending,
}

fn main() {
    if let Err(ref e) = run() {
        use std::io::Write;
        let stderr = &mut ::std::io::stderr();
        let errmsg = "Error writing to stderr";

        writeln!(stderr, "error: {}", e).expect(errmsg);

        for e in e.iter().skip(1) {
            writeln!(stderr, "caused by: {}", e).expect(errmsg);
        }

        // The backtrace is not always generated. Try to run this example
        // with `RUST_BACKTRACE=1`.
        if let Some(backtrace) = e.backtrace() {
            writeln!(stderr, "backtrace: {:?}", backtrace).expect(errmsg);
        }

        ::std::process::exit(1);
    }
}

pub fn run() -> Result<()> {
    let addr = format!("0.0.0.0:{}", 4567);

    let mut hub = Hub::new();
    hub.handle(
        "pull_request",
        |delivery: &Delivery| match delivery.payload {
            Event::PullRequest { ref pull_request, .. } => {
                let repo = &pull_request.head.repo.full_name;
                let branch = &pull_request.head._ref;
                let sha = &pull_request.head.sha;

                set_status(Status::Pending, repo, sha).expect("Can't set status to pending");
                let result = handle_pull_request(repo, branch);
                match result {
                    Err(e) => println!("An error occured during analysis: {}", e),
                    Ok(()) => println!("Pull request passed analysis"),
                }
            }
            _ => (),
        },
    );
    let srvc = Server::http(&addr[..]).unwrap().handle(hub);
    println!("listening on {}", addr);
    srvc?;
    Ok(())
}

fn set_status(status: Status, repo: &str, sha: &str) -> Result<reqwest::Response> {
    let token = env::var("GITHUB_TOKEN")?;
    let client = reqwest::Client::new();
    let mut params = HashMap::new();
    params.insert("state", "pending");
    params.insert("description", "Analysis started");
    Ok(client.request(
      reqwest::Method::Post,
        &format!(
        "https://api.github.com/repos/{}/statuses/{}?access_token={}",
        repo,
        sha,
        token),
    ).json(&params).send()?)
}

fn handle_pull_request(project_name: &str, branch: &str) -> Result<()> {
    // let readme_url = contents_url.replace("{+path}", "README.md");
    // let readme_url_branch = format!("{}?ref={}", readme_url, pull_request.head._ref);
    // println!("{}", readme_url_branch);
    // type: json
    // content: base 64
    // https://api.github.com/repos/mre/awesome-static-analysis/contents/README.md?ref=mre-patch-2
    // TODO: https://raw.githubusercontent.com/mre/awesome-static-analysis/master/README.md
    let mut payload = reqwest::get(&format!(
        "https://raw.githubusercontent.com/{}/{}/README.md",
        project_name,
        branch
    ))?;
    let mut result = String::new();
    payload.read_to_string(&mut result)?;
    println!("{}", result);
    check(result)

}

fn check_tool(tool: &str) -> Result<()> {
    println!(">{}<", tool);
    let captures = TOOL_REGEX.captures(tool).ok_or(
        "Invalid syntax for tool"
            .to_string(),
    )?;

    let name = captures[1].to_string();
    let link = captures[2].to_string();
    let desc = captures[4].to_string();

    println!("Desc: {}", desc);

    reqwest::get(&link)?;

    Ok(())
}

fn check_section(section: String) -> Result<()> {
    // Ignore license section
    if section.starts_with("License") {
        return Ok(());
    }

    // Skip section headline
    let lines: Vec<_> = section.split('\n').skip(1).collect();
    if lines.is_empty() {
        return Err(ErrorKind::EmptySection.into());
    };
    for line in lines {
        if line.is_empty() {
            continue;
        }
        // Exception for subsection headlines
        if !line.starts_with("*") && line.ends_with(":") &&
            SUBSECTION_HEADLINE_REGEX.is_match(line)
        {
            continue;
        }
        check_tool(line)?
    }
    Ok(())
}

fn check(text: String) -> Result<()> {
    let sections = text.split("\n# ");

    // Skip first two sections,
    // as they contain the prelude and the table of contents.
    for section in sections.skip(2) {
        let subsections = section.split("## ");
        for subsection in subsections.skip(1) {
            check_section(subsection.into())?;
        }
    }
    Ok(())
}
