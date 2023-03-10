use clap::Parser;
use itertools::Itertools;
use serde::Deserialize;
use serde_json::Value;
use std::{io::Write, path::Path};

#[derive(Parser, Debug)]
#[command(author="DocWilco <github@drwil.co>", version, about="Turn cache-domains git repo into VCL", long_about=None)]
struct Args {
    /// Directory where the repo is
    #[clap(short, long, default_value = "cachedomains")]
    repo_dir: String,
    /// Output file, defaults to stdout
    #[clap(short, long)]
    output: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CacheDomain {
    name: String,
    description: String,
    domain_files: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let repo_dir = Path::new(&args.repo_dir);

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
    let mut output: Box<dyn Write> = if let Some(ref output) = args.output {
        Box::new(std::fs::File::create(output).unwrap())
    } else {
        Box::new(std::io::stdout())
    };
    output.write_all(vcl.as_bytes()).unwrap();
    if let Some(output) = args.output {
        eprintln!("Wrote {}", output);
    }
}
