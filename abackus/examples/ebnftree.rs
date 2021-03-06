#![deny(warnings)]

struct Tokenizer<I: Iterator<Item=char>>(lexers::Scanner<I>);

impl<I: Iterator<Item=char>> Iterator for Tokenizer<I> {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.scan_whitespace();
        self.0.scan_math_op()
            .or_else(|| self.0.scan_number())
            .or_else(|| self.0.scan_identifier())
    }
}

fn tokenizer<I: Iterator<Item=char>>(input: I) -> Tokenizer<I> {
    Tokenizer(lexers::Scanner::new(input))
}

fn main() {
    let grammar = r#"
        expr   := expr ('+'|'-') term | term ;
        term   := term ('*'|'/') factor | factor ;
        factor := '-' factor | power ;
        power  := ufact '^' factor | ufact ;
        ufact  := ufact '!' | group ;
        group  := num | '(' expr ')' ;
    "#;

    let input = std::env::args().skip(1).
        collect::<Vec<String>>().join(" ");

    use std::str::FromStr;
    let trificator = abackus::ParserBuilder::default()
        .plug_terminal("num", |n| f64::from_str(n).is_ok())
        .sexprificator(&grammar, "expr");

    match trificator(&mut tokenizer(input.chars())) {
        Ok(trees) => for t in trees { println!("{}", t.print()); },
        Err(e) => println!("{:?}", e)
    }
}
