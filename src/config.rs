#[cfg(test)]
mod tests;

pub fn parse_config(args: &[String]) -> &str {
    let filename = &args.get(1)
        .expect("no file given");

    filename
}