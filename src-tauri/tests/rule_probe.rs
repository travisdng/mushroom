use mushroom_lib::ai::privacy::gitleaks_rules::GITLEAKS_RULES;
use std::time::Instant;

#[test]
fn how_big_does_the_set_need_to_be() {
    let patterns: Vec<&str> = GITLEAKS_RULES.iter().map(|r| r.pattern).collect();
    for mb in [10usize, 20, 40, 80, 160] {
        let started = Instant::now();
        let built = regex::RegexSetBuilder::new(&patterns)
            .size_limit(mb * 1024 * 1024)
            .build();
        match built {
            Ok(_) => {
                println!(
                    "limit {mb} MB -> OK in {} ms",
                    started.elapsed().as_millis()
                );
                break;
            }
            Err(_) => println!("limit {mb} MB -> too small"),
        }
    }

    // And what 220 individual regexes cost, as the alternative.
    let started = Instant::now();
    let each: Vec<regex::Regex> = GITLEAKS_RULES
        .iter()
        .map(|r| regex::Regex::new(r.pattern).unwrap())
        .collect();
    println!(
        "220 individual regexes -> {} compiled in {} ms",
        each.len(),
        started.elapsed().as_millis()
    );

    let without_keywords = GITLEAKS_RULES
        .iter()
        .filter(|r| r.keywords.is_empty())
        .count();
    println!("rules with no keyword prefilter: {without_keywords}");
}
