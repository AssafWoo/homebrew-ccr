use crate::runner::{ConvCompareResult, ConvFixtureResult, FixtureResult, QuestionResult};

pub fn print_fixture_result(r: &FixtureResult) {
    println!("  Lines:   {} → {} ({:.0}% reduction)",
        r.lines_in, r.lines_out,
        (1.0 - r.lines_out as f32 / r.lines_in as f32) * 100.0);
    println!("  Tokens:  {} → {} ({:.1}% saved)",
        r.input_tokens, r.output_tokens, r.savings_pct);
    println!("  Recall:  {:.0}% (compressed) vs {:.0}% (original baseline)",
        r.recall, r.original_recall);
    println!();

    for qr in &r.question_results {
        let orig_icon = if qr.original_score { "✓" } else { "✗" };
        let comp_icon = if qr.compressed_score { "✓" } else { "✗" };
        let status = match (qr.original_score, qr.compressed_score) {
            (true, true)  => "OK     ",
            (true, false) => "REGRESSION",
            (false, true) => "IMPROVED ",
            (false, false) => "BOTH_MISS",
        };
        println!("  [{}] Q: {}", status, qr.question);
        println!("      orig[{}]: {}", orig_icon, truncate(&qr.original_answer, 80));
        println!("      comp[{}]: {}", comp_icon, truncate(&qr.compressed_answer, 80));
        println!("      need one of: {:?}", qr.key_facts);
        println!();
    }
}

pub fn print_summary(results: &[FixtureResult]) {
    if results.is_empty() {
        println!("No fixtures ran.");
        return;
    }

    println!("=== SUMMARY ===");
    println!();
    println!("{:<25} {:>8} {:>8} {:>10} {:>10}",
        "Fixture", "Savings", "Lines↓", "Recall%", "Baseline%");
    println!("{}", "-".repeat(65));

    for r in results {
        let line_reduction = (1.0 - r.lines_out as f32 / r.lines_in as f32) * 100.0;
        println!("{:<25} {:>7.1}% {:>7.0}% {:>9.0}% {:>9.0}%",
            r.name, r.savings_pct, line_reduction, r.recall, r.original_recall);
    }

    println!("{}", "-".repeat(65));

    let avg_savings = results.iter().map(|r| r.savings_pct).sum::<f32>() / results.len() as f32;
    let avg_recall  = results.iter().map(|r| r.recall).sum::<f32>()  / results.len() as f32;
    let avg_baseline = results.iter().map(|r| r.original_recall).sum::<f32>() / results.len() as f32;

    println!("{:<25} {:>7.1}% {:>8} {:>9.0}% {:>9.0}%",
        "AVERAGE", avg_savings, "", avg_recall, avg_baseline);

    println!();

    // Count regressions
    let total_questions: usize = results.iter().map(|r| r.question_results.len()).sum();
    let regressions: usize = results.iter()
        .flat_map(|r| &r.question_results)
        .filter(|q| q.original_score && !q.compressed_score)
        .count();
    let improvements: usize = results.iter()
        .flat_map(|r| &r.question_results)
        .filter(|q| !q.original_score && q.compressed_score)
        .count();

    println!("Total questions:  {}", total_questions);
    println!("Regressions:      {} (worked in original, broke in compressed)", regressions);
    println!("Improvements:     {} (failed in original, works in compressed)", improvements);

    if regressions == 0 {
        println!();
        println!("✓ Zero regressions — compression did not hurt answer quality.");
    } else {
        println!();
        println!("⚠ {} regressions detected — review REGRESSION lines above.", regressions);
    }
}

pub fn print_conv_compare_result(r: &ConvCompareResult) {
    println!("  [{}]", r.description);
    println!("  Turns: {}", r.turns);
    println!();
    println!("  {:>12}  {:>10}  {:>9}  {:>10}  {:>8}  {:>8}",
        "", "Snapshot", "Snap Saved", "Cumulative", "Cum Saved", "Recall");
    println!("  {:>12}  {:>10}  {:>9}  {:>10}  {:>8}  {:>8}",
        "original", r.v1_tokens_in, "-", r.cumulative_tokens_original, "-",
        format!("{:.0}%", r.original_recall));
    println!("  {:>12}  {:>10}  {:>9}  {:>10}  {:>8}  {:>8}",
        "v1 (bert)", r.v1_tokens_out, format!("{:.1}%", r.v1_savings_pct),
        r.cumulative_tokens_v1, format!("{:.1}%", r.cumulative_savings_v1_pct),
        format!("{:.0}%", r.v1_recall));
    println!("  {:>12}  {:>10}  {:>9}  {:>10}  {:>8}  {:>8}",
        "v2 (ollama)", r.v2_tokens_out, format!("{:.1}%", r.v2_savings_pct),
        r.cumulative_tokens_v2, format!("{:.1}%", r.cumulative_savings_v2_pct),
        format!("{:.0}%", r.v2_recall));
    println!();

    for q in &r.questions {
        let v1_icon = if q.v1_score { "✓" } else { "✗" };
        let v2_icon = if q.v2_score { "✓" } else { "✗" };
        let status = match (q.v1_score, q.v2_score) {
            (true,  true)  => "OK      ",
            (true,  false) => "V2_REGR ",
            (false, true)  => "V2_IMPRV",
            (false, false) => "BOTH_MISS",
        };
        println!("  [{}] {}", status, q.question);
        println!("      v1[{}]: {}", v1_icon, truncate(&q.v1_answer, 80));
        println!("      v2[{}]: {}", v2_icon, truncate(&q.v2_answer, 80));
        println!("      need one of: {:?}", q.key_facts);
        println!();
    }
}

pub fn print_conv_compare_summary(results: &[ConvCompareResult]) {
    if results.is_empty() { return; }

    println!("=== V1 vs V2 COMPARISON SUMMARY ===");
    println!();
    println!("{:<26} {:>9} {:>9} {:>10} {:>10} {:>8} {:>8}",
        "Fixture", "V1 Snap", "V2 Snap", "V1 Cum", "V2 Cum", "V1 Rec%", "V2 Rec%");
    println!("{}", "-".repeat(90));

    for r in results {
        println!("{:<26} {:>8.1}% {:>8.1}% {:>9.1}% {:>9.1}% {:>7.0}% {:>7.0}%",
            r.name,
            r.v1_savings_pct, r.v2_savings_pct,
            r.cumulative_savings_v1_pct, r.cumulative_savings_v2_pct,
            r.v1_recall, r.v2_recall);
    }

    println!("{}", "-".repeat(90));

    let avg = |f: &dyn Fn(&ConvCompareResult) -> f32| -> f32 {
        results.iter().map(f).sum::<f32>() / results.len() as f32
    };
    let avg_v1_snap   = avg(&|r| r.v1_savings_pct);
    let avg_v2_snap   = avg(&|r| r.v2_savings_pct);
    let avg_v1_cum    = avg(&|r| r.cumulative_savings_v1_pct);
    let avg_v2_cum    = avg(&|r| r.cumulative_savings_v2_pct);
    let avg_v1_recall = avg(&|r| r.v1_recall);
    let avg_v2_recall = avg(&|r| r.v2_recall);

    println!("{:<26} {:>8.1}% {:>8.1}% {:>9.1}% {:>9.1}% {:>7.0}% {:>7.0}%",
        "AVERAGE", avg_v1_snap, avg_v2_snap, avg_v1_cum, avg_v2_cum, avg_v1_recall, avg_v2_recall);
    println!();

    let total_q: usize = results.iter().map(|r| r.questions.len()).sum();
    let v2_regressions: usize = results.iter()
        .flat_map(|r| &r.questions)
        .filter(|q| q.v1_score && !q.v2_score)
        .count();
    let v2_improvements: usize = results.iter()
        .flat_map(|r| &r.questions)
        .filter(|q| !q.v1_score && q.v2_score)
        .count();

    println!("Total questions:   {}", total_q);
    println!("V2 regressions:    {} (v1 passed, v2 failed)", v2_regressions);
    println!("V2 improvements:   {} (v1 failed, v2 passed)", v2_improvements);

    if v2_regressions == 0 {
        println!();
        println!("✓ V2 introduced zero regressions vs V1.");
    } else {
        println!();
        println!("⚠ {} V2 regressions — review V2_REGR lines above.", v2_regressions);
    }
}

pub fn print_conv_fixture_result(r: &ConvFixtureResult) {
    println!("  [{}]", r.description);
    println!("  Turns:   {}", r.turns);
    println!("  Tokens:  {} → {} ({:.1}% saved)", r.tokens_in, r.tokens_out, r.savings_pct);
    println!("  Recall:  {:.0}% (compressed) vs {:.0}% (original baseline)",
        r.recall, r.original_recall);
    println!();

    for qr in &r.question_results {
        let orig_icon = if qr.original_score { "✓" } else { "✗" };
        let comp_icon = if qr.compressed_score { "✓" } else { "✗" };
        let status = match (qr.original_score, qr.compressed_score) {
            (true, true)   => "OK        ",
            (true, false)  => "REGRESSION",
            (false, true)  => "IMPROVED  ",
            (false, false) => "BOTH_MISS ",
        };
        println!("  [{}] Q: {}", status, qr.question);
        println!("      orig[{}]: {}", orig_icon, truncate(&qr.original_answer, 80));
        println!("      comp[{}]: {}", comp_icon, truncate(&qr.compressed_answer, 80));
        println!("      need one of: {:?}", qr.key_facts);
        println!();
    }
}

pub fn print_conv_summary(results: &[ConvFixtureResult]) {
    if results.is_empty() {
        return;
    }

    println!("=== CONVERSATION COMPRESSION SUMMARY ===");
    println!();
    println!("{:<30} {:>8} {:>8} {:>10} {:>10}",
        "Fixture", "Savings", "Turns", "Recall%", "Baseline%");
    println!("{}", "-".repeat(70));

    for r in results {
        println!("{:<30} {:>7.1}% {:>8} {:>9.0}% {:>9.0}%",
            r.name, r.savings_pct, r.turns, r.recall, r.original_recall);
    }

    println!("{}", "-".repeat(70));

    let avg_savings  = results.iter().map(|r| r.savings_pct).sum::<f32>() / results.len() as f32;
    let avg_recall   = results.iter().map(|r| r.recall).sum::<f32>()      / results.len() as f32;
    let avg_baseline = results.iter().map(|r| r.original_recall).sum::<f32>() / results.len() as f32;

    println!("{:<30} {:>7.1}% {:>8} {:>9.0}% {:>9.0}%",
        "AVERAGE", avg_savings, "", avg_recall, avg_baseline);
    println!();

    let total_questions: usize = results.iter().map(|r| r.question_results.len()).sum();
    let regressions: usize = results.iter()
        .flat_map(|r| &r.question_results)
        .filter(|q| q.original_score && !q.compressed_score)
        .count();
    let improvements: usize = results.iter()
        .flat_map(|r| &r.question_results)
        .filter(|q| !q.original_score && q.compressed_score)
        .count();

    println!("Total questions:  {}", total_questions);
    println!("Regressions:      {} (worked in original, broke in compressed)", regressions);
    println!("Improvements:     {} (failed in original, works in compressed)", improvements);

    if regressions == 0 {
        println!();
        println!("✓ Zero regressions — conversation compression preserved all key facts.");
    } else {
        println!();
        println!("⚠ {} regressions — review REGRESSION lines above.", regressions);
    }
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ");
    if s.len() <= max {
        s
    } else {
        format!("{}…", &s[..max])
    }
}
