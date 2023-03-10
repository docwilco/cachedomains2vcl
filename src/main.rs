use clap::Parser;
use itertools::Itertools;
use serde::Deserialize;
use serde_json::Value;
use std::{io::Write, path::Path, process::Command};

#[derive(Parser, Debug)]
#[command(author="DocWilco <github@drwil.co>", version, about="Turn cache-domains git repo into VCL", long_about=None)]
struct Args {
    /// The git repo to clone
    #[clap(
        short,
        long,
        default_value = "https://github.com/uklans/cache-domains.git"
    )]
    repo: String,
    /// Work directory, where the repo will be cloned to and cachedomains.vcl
    /// will be written to
    #[clap(short, long, default_value = "cachedomains")]
    work_dir: String,
}

#[derive(Deserialize, Debug)]
struct CacheDomain {
    name: String,
    description: String,
    domain_files: Vec<String>,
}

fn main() {
    let args = Args::parse();

    // Create work directory if it doesn't exist
    let work_dir = Path::new(&args.work_dir);
    if !Path::new(&work_dir).exists() {
        std::fs::create_dir(&work_dir).unwrap();
    }
    // Create repo directory if it doesn't exist
    let repo_dir = work_dir.join("repo");
    let repo_exists = repo_dir.exists();
    if !repo_exists {
        std::fs::create_dir(&repo_dir).unwrap();
    }

    // If repo directory existed already, try a git pull, if that fails,
    // or if the repo directory didn't exist, try a git clone.
    let pull_failed_or_dir_didnt_exist = !repo_exists || {
        println!("repo directory exists, trying git pull");
        let mut git_pull = Command::new("git")
            .arg("pull")
            .current_dir(&repo_dir)
            .spawn()
            .expect("failed to execute process");
        let git_pull_status = git_pull.wait().expect("failed to wait on child");
        if !git_pull_status.success() {
            println!("git pull failed");
        }
        !git_pull_status.success()
    };
    if pull_failed_or_dir_didnt_exist {
        println!("trying git clone");
        let mut git_clone = Command::new("git")
            .arg("clone")
            .arg(args.repo)
            .arg(&repo_dir)
            .spawn()
            .expect("failed to execute process");
        let git_clone_status = git_clone.wait().expect("failed to wait on child");
        if !git_clone_status.success() {
            panic!("git clone failed");
        }
    }

    // Read cache_domains.json
    let json_file = repo_dir.join("cache_domains.json");
    let mut cache_domains: Value =
        serde_json::from_str(&std::fs::read_to_string(json_file).unwrap()).unwrap();
    let cache_domains: Vec<CacheDomain> = serde_json::from_value(
        cache_domains
            .as_object_mut()
            .unwrap()
            .get_mut("cache_domains")
            .unwrap()
            .take(),
    )
    .unwrap();

    // Generate VCL
    let mut vcl = String::new();
    vcl.push_str(
        r#"/* generated, do not edit */
sub set_cache_domain {
    unset req.http.x-cache-domain;
    if (req.http.user-agent ~ "Valve/Steam HTTP Client 1\.0") {
        /* This agent is a special case, use steam as cache domain
         * regardless of the hostname. */
        set req.http.x-cache-domain = "steam";
"#,
    );
    for cache_domain in cache_domains {
        // load domain files
        let host_regexes = cache_domain
            .domain_files
            .iter()
            .flat_map(|domain_file| {
                let domain_path = repo_dir.join(domain_file);
                let domains = std::fs::read_to_string(domain_path)
                    .expect(&format!("Unable to read file: {}", domain_file));
                domains
                    .lines() // for each line
                    .map(|hostname| {
                        // remove comments
                        let hostname = hostname.split('#').next().unwrap();
                        // remove whitespace
                        let hostname = hostname.trim();
                        // replace . with \.
                        let regex = hostname.replace(".", "\\.");
                        // replace * with .*
                        let regex = regex.replace("*", ".*");
                        regex
                    })
                    // remove empty lines
                    .filter(|hostname| !hostname.is_empty())
                    .collect::<Vec<String>>()
            })
            // combine all the regexes into one
            .join("|");
        // Surround with ^( and )$
        let host_regexes = format!("^({})$", host_regexes);
        vcl.push_str(r#"    } else if (req.http.host ~ ""#);
        vcl.push_str(&host_regexes);
        vcl.push_str(
            r#"") {
        // "#,
        );
        vcl.push_str(&cache_domain.description);
        vcl.push_str(
            r#"
        set req.http.x-cache-domain = ""#,
        );
        vcl.push_str(&cache_domain.name);
        vcl.push_str(
            r#"";
"#,
        );
    }
    vcl.push_str(
        r#"    }
}"#,
    );
    // Open output file for writing
    let vcl_file = work_dir.join("cachedomains.vcl");
    let mut output_file = std::fs::File::create(vcl_file).unwrap();
    output_file.write_all(vcl.as_bytes()).unwrap();
    println!("Wrote cache_domains/cache_domains.vcl")
}
