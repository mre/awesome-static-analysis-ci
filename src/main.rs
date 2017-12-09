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
use std::fmt;
use std::cmp::Ordering;

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
    }
}

enum Status {
    Success,
    Pending,
    Failure,
    Error,
}

struct Tool {
    name: String,
    link: String,
    desc: String,
}

impl Tool {
    fn new<T: Into<String>>(name: T, link: T, desc: T) -> Self {
        Tool {
            name: name.into(),
            link: link.into(),
            desc: desc.into(),
        }
    }
}

impl PartialEq for Tool {
    fn eq(&self, other: &Tool) -> bool {
        self.name.to_lowercase() == other.name.to_lowercase()
    }
}

impl Eq for Tool {}

impl PartialOrd for Tool {
    fn partial_cmp(&self, other: &Tool) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Tool {
    fn cmp(&self, other: &Tool) -> Ordering {
        self.name.to_lowercase().cmp(&other.name.to_lowercase())
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Status::Pending => write!(f, "pending"),
            Status::Success => write!(f, "success"),
            Status::Failure => write!(f, "failure"),
            Status::Error => write!(f, "error"),
        }
    }
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
    // Check that the API token is available
    env::var("GITHUB_TOKEN")?;

    let addr = format!("0.0.0.0:{}", 4567);

    let mut hub = Hub::new();
    hub.handle(
        "pull_request",
        |delivery: &Delivery| match delivery.payload {
            Event::PullRequest { ref action, ref pull_request, .. } => {
                match action.as_ref() {
                    "opened" | "reopened" | "edited" | "synchronized" => (),
                    _ => return ()
                }
                let base_repo = &pull_request.base.repo.full_name;
                let head_repo = &pull_request.head.repo.full_name;
                let head_branch = &pull_request.head._ref;
                let head_sha = &pull_request.head.sha;

                set_status(Status::Pending, "Analysis started".into(), base_repo, head_sha).expect("Can't set status to pending");
                let result = handle_pull_request(head_repo, head_branch);
                println!("{:#?}", result);

                let mut response = match result {
                    Ok(()) => set_status(Status::Success, "Build successful".into(), base_repo, head_sha).expect("Can't set status to success"),
                    Err(e) => {
                        let mut desc = e.description().to_string();
                        desc.truncate(140);
                        println!("{}", desc);
                        set_status(Status::Error, desc, base_repo, head_sha).expect("Can't set status to error")
                    },
                };

                println!("Sent final status. Response code: {}", response.status());
                let mut buf = String::new();
                response.read_to_string(&mut buf).expect("Failed to read response");
                println!("Response body: {}", buf);
            }
            _ => (),
        },
    );
    let srvc = Server::http(&addr[..]).unwrap().handle(hub);
    println!("listening on {}", addr);
    srvc?;
    Ok(())
}

fn set_status(status: Status, desc: String, repo: &str, sha: &str) -> Result<reqwest::Response> {
    let token = env::var("GITHUB_TOKEN")?;
    let client = reqwest::Client::new();
    let mut params = HashMap::new();
    params.insert("state", format!("{}", status));
    params.insert("description", desc);
    println!("Sending status: {:#?}", params);

    let status_url = format!("https://api.github.com/repos/{}/statuses/{}", repo, sha);
    println!("Status url: {}", status_url);
    Ok(client
        .request(
            reqwest::Method::Post,
            &format!(
                "{}?access_token={}",
                status_url,
                token,
            ),
        )
        .json(&params)
        .send()?)
}

fn handle_pull_request(project_name: &str, branch: &str) -> Result<()> {
    let mut payload = reqwest::get(&format!(
        "https://raw.githubusercontent.com/{}/{}/README.md",
        project_name,
        branch
    ))?;
    let mut result = String::new();
    payload.read_to_string(&mut result)?;
    check(result)
}

fn check_tool(tool: &str) -> Result<Tool> {
    println!("Checking `{}`", tool);
    let captures = TOOL_REGEX.captures(tool).ok_or(format!(
        "Invalid syntax for tool: {}",
        tool
    ))?;

    let name = captures[1].to_string();
    let link = captures[2].to_string();
    let desc = captures[4].to_string();

    if name.len() > 50 {
        bail!("Name of tool is suspiciously long: `{}`", name);
    }

    // A somewhat arbitrarily chosen description length.
    // Note that this includes any markdown formatting
    // like links. Therefore we are quite generous for now.
    if desc.len() > 200 {
        bail!("Desription of `{}` is too long: {}", name, desc);
    }

    reqwest::get(&link)?;

    Ok(Tool::new(name, link, desc))
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

    let mut tools = vec![];
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
        tools.push(check_tool(line)?);
    }
    // Tools need to be alphabetically ordered
    check_ordering(tools)
}

fn check_ordering(tools: Vec<Tool>) -> Result<()> {
    match tools.windows(2).find(|t| t[0] > t[1]) {
        Some(tools) => bail!("`{}` does not conform to alphabetical ordering", tools[0].name),
        None => Ok(()),
    }
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

mod tests {
    use super::*;

    #[test]
    fn test_ordering() {
        assert!(check_ordering(vec![]).is_ok());

        assert!(check_ordering(vec![Tool::new("a", "url", "desc")]).is_ok());

        assert!(
            check_ordering(vec![
                Tool::new("0", "", ""),
                Tool::new("1", "", ""),
                Tool::new("a", "", ""),
                Tool::new("Axx", "", ""),
                Tool::new("B", "", ""),
                Tool::new("b", "", ""),
                Tool::new("c", "", ""),
            ]).is_ok()
        );

        assert!(
            check_ordering(vec![
                Tool::new("b", "", ""),
                Tool::new("a", "", ""),
                Tool::new("c", "", ""),
            ]).is_err()
        );
    }
}
