# cachedomains2vcl

cachedomains2vcl is a tool to generate Varnish VCL from a list of domains for
use in `docwilco/lancache-monolithic-varnish.`

## Usage

```bash
$ cachedomains2vcl -h
Turn cachedomains git repo into VCL

Usage: cachedomains2vcl.exe [OPTIONS]

Options:
  -r, --repo <REPO>          The git repo to clone [default: https://github.com/uklans/cache-domains.git]
  -w, --work-dir <WORK_DIR>  Work directory, where the repo will be cloned to and cachedomains.vcl will be written to [default: cachedomains]
  -h, --help                 Print help
  -V, --version              Print version
```