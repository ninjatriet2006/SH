fn main() {
    let resp = ureq::get("http://worldtimeapi.org/api/timezone/Etc/UTC").call().unwrap();
    let body = resp.into_string().unwrap();
    println!("{}", body);
}
