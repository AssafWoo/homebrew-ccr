//! Handler benchmark tests — realistic large-project fixtures.
//!
//! Each benchmark feeds a realistic command output through the handler and compares
//! token counts before (what Claude sees without CCR) and after (what Claude sees with CCR).
//!
//! Run with:
//!   cargo test -p ccr benchmark -- --nocapture
//!
//! For git status / git log / cargo build the "without CCR" baseline is the command's
//! native verbose output; the handler receives the flag-rewritten form (porcelain /
//! oneline / --message-format json) and compresses it further. The combination is
//! the true end-to-end savings a user gets after `ccr init`.

use ccr::handlers::{cargo::CargoHandler, git::GitHandler, jest::JestHandler, ls::LsHandler, tsc::TscHandler, Handler};
use ccr_core::tokens::count_tokens;

// ─── helpers ─────────────────────────────────────────────────────────────────

fn savings_pct(in_tok: usize, out_tok: usize) -> f64 {
    if in_tok == 0 { return 0.0; }
    (in_tok - out_tok) as f64 / in_tok as f64 * 100.0
}

fn run(handler: &dyn Handler, handler_input: &str, args: &[&str]) -> String {
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    handler.filter(handler_input, &args)
}

// ─── fixtures ────────────────────────────────────────────────────────────────

/// `cargo build` with 130 real crate names and 5 warnings.
/// baseline  = human-readable stdout (what Claude sees without CCR)
/// handler_input = --message-format json (what filter() receives after arg rewrite)
fn cargo_build() -> (String /* baseline */, String /* handler_input */) {
    let deps = [
        "proc-macro2","unicode-ident","syn","quote","serde","serde_derive",
        "serde_json","itoa","ryu","indexmap","hashbrown","ahash","zerocopy",
        "once_cell","lazy_static","regex","regex-syntax","aho-corasick","memchr",
        "bytes","tokio","tokio-macros","mio","socket2","libc","pin-project-lite",
        "futures-core","futures-util","futures-sink","futures-task","pin-utils",
        "slab","async-trait","tower","tower-layer","tower-service",
        "hyper","http","http-body","httparse","h2","want",
        "tracing","tracing-core","tracing-subscriber","tracing-attributes","valuable",
        "log","env_logger","humantime","termcolor","atty",
        "clap","clap_derive","clap_lex","clap_builder","strsim",
        "anstream","anstyle","anstyle-parse","anstyle-query","colorchoice","utf8parse",
        "anyhow","thiserror","thiserror-impl","dirs","dirs-sys",
        "cfg-if","bitflags","nix","rustix","linux-raw-sys","errno",
        "tempfile","rand","rand_core","rand_chacha","ppv-lite86","getrandom",
        "uuid","hex","base64","url","percent-encoding","idna",
        "unicode-normalization","unicode-bidi","form_urlencoded","tinyvec",
        "reqwest","rustls","rustls-webpki","ring","spin","untrusted",
        "openssl","openssl-sys","foreign-types","foreign-types-shared",
        "native-tls","security-framework","security-framework-sys",
        "core-foundation","core-foundation-sys","cc","pkg-config",
        "chrono","num-integer","num-traits","iana-time-zone",
        "time","time-macros","deranged","powerfmt",
        "sqlx","sqlx-core","sqlx-macros","sqlx-postgres","sqlx-sqlite","dotenvy","heck",
        "axum","axum-core","matchit","mime","mime_guess","encoding_rs",
        "tower-http","hyper-util","http-body-util",
        "myapp",
    ];

    // ── baseline: human-readable (no flag rewriting) ──
    let mut baseline = String::new();
    for dep in &deps {
        baseline.push_str(&format!("   Compiling {} v1.0.0\n", dep));
    }
    baseline.push_str(concat!(
        "warning: unused variable: `conn`\n",
        " --> src/db/pool.rs:87:9\n",
        "  |\n",
        "87|     let conn = pool.acquire().await?;\n",
        "  |         ^^^^ help: if this is intentional, prefix it with an underscore: `_conn`\n",
        "  |\n",
        "  = note: `#[warn(unused_variables)]` on by default\n\n",
        "warning: unused variable: `config`\n",
        " --> src/server.rs:23:9\n",
        "  |\n",
        "23|     let config = AppConfig::load()?;\n",
        "  |         ^^^^^^ help: prefix with an underscore: `_config`\n\n",
        "warning: unused variable: `req`\n",
        " --> src/middleware/auth.rs:45:9\n",
        "  |\n",
        "45|     let req = request.into_parts();\n",
        "  |         ^^^ help: prefix with an underscore: `_req`\n\n",
        "warning: function is never used: `legacy_handler`\n",
        " --> src/api/v1.rs:120:4\n",
        "  |\n",
        "120| fn legacy_handler() {}\n",
        "   | ^^^^^^^^^^^^^^^^^^^^^^\n",
        "  |\n",
        "  = note: `#[warn(dead_code)]` on by default\n\n",
        "warning: unreachable expression\n",
        " --> src/handlers/webhook.rs:67:9\n",
        "  |\n",
        "67|     return Ok(());\n",
        "68|     log::info!(\"done\");\n",
        "   |     ^^^^^^^^^^^^^^^^^ unreachable expression\n\n",
        "warning: `myapp` (bin \"myapp\") generated 5 warnings\n",
        "    Finished `dev` profile [unoptimized + debuginfo] target(s) in 87.45s\n",
    ));

    // ── handler_input: --message-format json ──
    let mut h = String::new();
    for dep in deps.iter().take(deps.len() - 1) {
        h.push_str(&format!(
            "{{\"reason\":\"compiler-artifact\",\"package_id\":\"{dep} 1.0.0 \
             (registry+https://github.com/rust-lang/crates.io-index)\",\
             \"target\":{{\"kind\":[\"lib\"],\"name\":\"{dep}\",\"src_path\":\
             \"/home/user/.cargo/registry/src/{dep}/src/lib.rs\"}},\
             \"profile\":{{\"opt_level\":\"0\",\"debuginfo\":2}},\
             \"features\":[],\"filenames\":[\"/path/to/lib{dep}.rlib\"],\"fresh\":false}}\n",
            dep = dep
        ));
    }
    let warnings = [
        ("unused_variables", "unused variable: `conn`",              "src/db/pool.rs",          87),
        ("unused_variables", "unused variable: `config`",            "src/server.rs",           23),
        ("unused_variables", "unused variable: `req`",               "src/middleware/auth.rs",  45),
        ("dead_code",        "function is never used: `legacy_handler`", "src/api/v1.rs",       120),
        ("unreachable_code", "unreachable expression",               "src/handlers/webhook.rs", 67),
    ];
    for (code, msg, file, line) in &warnings {
        h.push_str(&format!(
            "{{\"reason\":\"compiler-message\",\
             \"package_id\":\"myapp 0.1.0 (path+file:///path/to/myapp)\",\
             \"target\":{{\"kind\":[\"bin\"],\"name\":\"myapp\"}},\
             \"message\":{{\"message\":\"{msg}\",\"level\":\"warning\",\
             \"spans\":[{{\"file_name\":\"{file}\",\"line_start\":{line}}}],\
             \"code\":{{\"code\":\"{code}\"}},\"rendered\":\"warning: {msg}...\"}}}}\n",
            msg = msg, file = file, line = line, code = code
        ));
    }
    h.push_str("{\"reason\":\"build-finished\",\"success\":true}\n");

    (baseline, h)
}

/// `cargo test` — 198 passing, 2 failures.
fn cargo_test() -> String {
    let mut out = String::new();
    let modules = [
        "api", "auth", "db", "handlers", "middleware",
        "models", "utils", "config", "services",
    ];
    let mut n = 0usize;
    for module in &modules {
        for i in 0..25usize {
            out.push_str(&format!(
                "test {}::tests::test_{}_case_{:02} ... ok\n", module, module, i
            ));
            n += 1;
            if n >= 198 { break; }
        }
        if n >= 198 { break; }
    }
    out.push_str("test auth::tests::test_jwt_expiry ... FAILED\n");
    out.push_str("test db::tests::test_pool_overflow ... FAILED\n");
    out.push_str("\nfailures:\n\n");
    out.push_str("---- auth::tests::test_jwt_expiry stdout ----\n");
    out.push_str("thread 'auth::tests::test_jwt_expiry' panicked at \
                  'assertion failed: token.is_valid()'\n");
    out.push_str("src/auth/jwt.rs:156:9\n");
    out.push_str("note: run with `RUST_BACKTRACE=1` for a backtrace\n\n");
    out.push_str("---- db::tests::test_pool_overflow stdout ----\n");
    out.push_str("thread 'db::tests::test_pool_overflow' panicked at \
                  'called `Result::unwrap()` on an `Err` value: PoolTimedOut'\n");
    out.push_str("src/db/pool.rs:89:14\n\n");
    out.push_str("failures:\n");
    out.push_str("    auth::tests::test_jwt_expiry\n");
    out.push_str("    db::tests::test_pool_overflow\n\n");
    out.push_str(
        "test result: FAILED. 198 passed; 2 failed; 0 ignored; finished in 14.32s\n",
    );
    out
}

/// `git status` — verbose baseline + porcelain handler input.
fn git_status() -> (String /* baseline */, String /* porcelain */) {
    let staged = [
        "src/auth/login.ts", "src/auth/logout.ts", "src/auth/middleware.ts",
        "src/api/users.ts",  "src/api/posts.ts",   "src/api/comments.ts",
        "src/models/user.ts","src/models/post.ts", "src/services/auth.ts",
        "src/config/database.ts",
    ];
    let modified = [
        "src/api/health.ts",         "src/api/metrics.ts",
        "src/components/Button.tsx", "src/components/Modal.tsx",
        "src/components/Form.tsx",   "src/components/Table.tsx",
        "src/components/Header.tsx", "src/components/Footer.tsx",
        "src/components/Sidebar.tsx","src/components/Dashboard.tsx",
        "src/pages/Home.tsx",        "src/pages/Login.tsx",
        "src/pages/Register.tsx",    "src/pages/Profile.tsx",
        "src/pages/Settings.tsx",    "src/hooks/useAuth.ts",
        "src/hooks/useUser.ts",      "src/hooks/usePosts.ts",
        "src/store/auth.ts",         "src/store/posts.ts",
        "src/store/ui.ts",           "src/utils/api.ts",
        "src/utils/format.ts",       "src/utils/validate.ts",
        "src/utils/storage.ts",      "src/utils/errors.ts",
        "tests/auth.test.ts",        "tests/api.test.ts",
        "tests/components.test.tsx", "package.json",
        "tsconfig.json",             "jest.config.ts",
        "src/styles/globals.css",    "src/styles/components.css",
        "src/constants/routes.ts",   "src/constants/api.ts",
        "src/types/user.ts",         "src/types/post.ts",
        "src/types/api.ts",          "src/types/ui.ts",
    ];
    let untracked = [
        "src/components/NewWidget.tsx",
        "src/pages/Analytics.tsx",
        "src/hooks/useAnalytics.ts",
        "src/utils/logger.ts",
        "src/services/analytics.ts",
        "src/types/analytics.ts",
        "migrations/20240318_add_analytics.sql",
        "docs/ANALYTICS.md",
    ];

    let mut baseline = String::new();
    baseline.push_str("On branch feature/user-auth\n");
    baseline.push_str("Your branch is ahead of 'origin/feature/user-auth' by 3 commits.\n");
    baseline.push_str("  (use \"git push\" to publish your local commits)\n\n");
    baseline.push_str("Changes to be committed:\n");
    baseline.push_str("  (use \"git restore --staged <file>...\" to unstage)\n");
    for f in &staged   { baseline.push_str(&format!("\tmodified:   {}\n", f)); }
    baseline.push_str("\nChanges not staged for commit:\n");
    baseline.push_str("  (use \"git restore <file>...\" to update what will be committed)\n");
    baseline.push_str("  (use \"git add <file>...\" to update what will be committed)\n");
    for f in &modified { baseline.push_str(&format!("\tmodified:   {}\n", f)); }
    baseline.push_str("\nUntracked files:\n");
    baseline.push_str("  (use \"git add <file>...\" to include in what will be committed)\n");
    for f in &untracked { baseline.push_str(&format!("\t{}\n", f)); }

    let mut porcelain = String::new();
    for f in &staged    { porcelain.push_str(&format!("M  {}\n", f)); }
    for f in &modified  { porcelain.push_str(&format!(" M {}\n", f)); }
    for f in &untracked { porcelain.push_str(&format!("?? {}\n", f)); }

    (baseline, porcelain)
}

/// `git log` — full verbose baseline + --oneline handler input, 25 commits.
fn git_log() -> (String /* verbose */, String /* oneline */) {
    let commits = [
        ("a1b2c3d", "feat: add user authentication middleware with JWT support"),
        ("e4f5g6h", "fix: resolve session token expiry edge case in auth service"),
        ("i7j8k9l", "refactor: extract database connection pool into separate module"),
        ("m0n1o2p", "feat: implement rate limiting for API endpoints"),
        ("q3r4s5t", "fix: correct pagination offset calculation in list endpoints"),
        ("u6v7w8x", "chore: update dependencies to latest stable versions"),
        ("y9z0a1b", "feat: add Redis cache layer for frequently accessed data"),
        ("c2d3e4f", "test: add integration tests for authentication flows"),
        ("g5h6i7j", "fix: handle null values in user profile update endpoint"),
        ("k8l9m0n", "feat: implement webhook delivery with exponential retry logic"),
        ("o1p2q3r", "refactor: consolidate error handling into shared middleware"),
        ("s4t5u6v", "fix: resolve race condition in concurrent request handler"),
        ("w7x8y9z", "feat: add audit logging for all sensitive data operations"),
        ("a0b1c2d", "chore: add GitHub Actions workflow for CI/CD pipeline"),
        ("e3f4g5h", "fix: correct CORS headers for cross-origin preflight requests"),
        ("i6j7k8l", "feat: implement file upload service with S3 integration"),
        ("m9n0o1p", "test: expand unit test coverage for all database models"),
        ("q2r3s4t", "fix: resolve memory leak in long-running background jobs"),
        ("u5v6w7x", "refactor: migrate all configuration to environment variables"),
        ("y8z9a0b", "feat: add Prometheus metrics endpoint for cluster monitoring"),
        ("c1d2e3f", "fix: handle graceful shutdown for all in-flight requests"),
        ("g4h5i6j", "docs: update API documentation with newly added endpoints"),
        ("k7l8m9n", "feat: implement automated database migration runner"),
        ("o0p1q2r", "fix: correct timestamp timezone handling in all API responses"),
        ("s3t4u5v", "chore: initial project setup with core dependency configuration"),
    ];
    let authors = [
        ("Alice Johnson", "alice@company.com"),
        ("Bob Smith",     "bob@company.com"),
        ("Carol White",   "carol@company.com"),
        ("David Brown",   "david@company.com"),
        ("Eve Martinez",  "eve@company.com"),
    ];
    let dates = [
        "Mon Mar 18 14:32:10 2024 +0000", "Fri Mar 15 10:15:42 2024 +0000",
        "Thu Mar 14 16:47:33 2024 +0000", "Wed Mar 13 09:22:18 2024 +0000",
        "Tue Mar 12 14:55:07 2024 +0000", "Mon Mar 11 11:30:59 2024 +0000",
        "Fri Mar  8 17:04:21 2024 +0000", "Thu Mar  7 13:18:44 2024 +0000",
        "Wed Mar  6 10:42:35 2024 +0000", "Tue Mar  5 15:29:16 2024 +0000",
        "Mon Mar  4 09:50:08 2024 +0000", "Fri Mar  1 16:37:52 2024 +0000",
        "Thu Feb 29 12:14:29 2024 +0000", "Wed Feb 28 09:45:11 2024 +0000",
        "Tue Feb 27 14:23:47 2024 +0000", "Mon Feb 26 11:06:33 2024 +0000",
        "Fri Feb 23 17:42:20 2024 +0000", "Thu Feb 22 13:55:04 2024 +0000",
        "Wed Feb 21 10:18:49 2024 +0000", "Tue Feb 20 15:01:37 2024 +0000",
        "Mon Feb 19 09:34:22 2024 +0000", "Fri Feb 16 16:47:15 2024 +0000",
        "Thu Feb 15 12:20:58 2024 +0000", "Wed Feb 14 09:03:41 2024 +0000",
        "Tue Feb 13 14:36:24 2024 +0000",
    ];

    let mut verbose = String::new();
    let mut oneline = String::new();
    for (i, (short_hash, msg)) in commits.iter().enumerate() {
        let (author, email) = authors[i % authors.len()];
        let date = dates[i];
        let full_hash = format!("{}abc123def456abc123def456abc123def456", short_hash);
        verbose.push_str(&format!(
            "commit {}\nAuthor: {} <{}>\nDate:   {}\n\n    {}\n\n",
            full_hash, author, email, date, msg
        ));
        oneline.push_str(&format!("{} {}\n", short_hash, msg));
    }
    (verbose, oneline)
}

/// `git diff` — five-file feature-branch diff with realistic 3-line context per hunk.
/// Real `git diff` uses -U3 (3 context lines before and after each change) by default.
/// The handler keeps structural lines + change lines + up to 2 context lines *after* a change;
/// it drops all context lines *before* a change. This becomes significant at scale.
fn git_diff() -> String {
    // Helper: build a realistic hunk with 3-line context before/after each change block.
    // Returns a String that looks exactly like `git diff -U3` output.
    fn hunk(before_start: u32, after_start: u32, context_before: &[&str],
            changes: &[(&str, &str)], context_after: &[&str]) -> String {
        // +/- lines interleaved
        let removed: Vec<&str> = changes.iter().map(|(r, _)| *r).filter(|s| !s.is_empty()).collect();
        let added:   Vec<&str> = changes.iter().map(|(_, a)| *a).filter(|s| !s.is_empty()).collect();
        let before_len = context_before.len() as u32 + removed.len() as u32 + context_after.len() as u32;
        let after_len  = context_before.len() as u32 + added.len()   as u32 + context_after.len() as u32;
        let mut s = format!("@@ -{},{} +{},{} @@\n", before_start, before_len, after_start, after_len);
        for c in context_before { s.push_str(&format!(" {}\n", c)); }
        for (rem, add) in changes {
            if !rem.is_empty() { s.push_str(&format!("-{}\n", rem)); }
            if !add.is_empty() { s.push_str(&format!("+{}\n", add)); }
        }
        for c in context_after { s.push_str(&format!(" {}\n", c)); }
        s
    }

    let mut out = String::new();

    // ── file 1: src/auth/middleware.ts ──────────────────────────────────────
    out.push_str("diff --git a/src/auth/middleware.ts b/src/auth/middleware.ts\n");
    out.push_str("index a1b2c3d..e4f5g6h 100644\n");
    out.push_str("--- a/src/auth/middleware.ts\n");
    out.push_str("+++ b/src/auth/middleware.ts\n");
    out.push_str(&hunk(1, 1,
        &["import { Request, Response, NextFunction } from 'express';",
          "import jwt from 'jsonwebtoken';",
          "import { config } from '../config';"],
        &[("", "import { logger } from '../utils/logger';"),
          ("", "import { AppError } from '../utils/errors';")],
        &["", "export function authenticate(req: Request, res: Response, next: NextFunction) {",
          "  const token = req.headers.authorization?.split(' ')[1];"],
    ));
    out.push_str(&hunk(10, 14,
        &["  const token = req.headers.authorization?.split(' ')[1];",
          "  if (!token) {",
          "    // no token"],
        &[("    return res.status(401).json({ error: 'No token provided' });",
           "    logger.warn('Request without authentication token', { path: req.path });"),
          ("", "    throw new AppError('Authentication required', 401);")],
        &["  }", "  try {", "    const decoded = jwt.verify(token, config.jwtSecret);"],
    ));
    out.push_str(&hunk(20, 26,
        &["    const decoded = jwt.verify(token, config.jwtSecret);",
          "    req.user = decoded as AuthUser;",
          "    // proceed"],
        &[("", "    logger.debug('Token verified', { userId: (decoded as AuthUser).id });")],
        &["    next();", "  } catch (error) {", "    // token invalid"],
    ));
    out.push_str(&hunk(27, 34,
        &["  } catch (error) {", "    // token invalid", "    // reject"],
        &[("    return res.status(401).json({ error: 'Invalid token' });",
           "    if (error instanceof jwt.TokenExpiredError) {"),
          ("", "      throw new AppError('Token has expired', 401);"),
          ("", "    }"),
          ("", "    logger.error('Token verification failed', { error });"),
          ("", "    throw new AppError('Invalid authentication token', 401);")],
        &["  }", "}"],
    ));

    // ── file 2: src/api/users.ts ────────────────────────────────────────────
    out.push_str("\ndiff --git a/src/api/users.ts b/src/api/users.ts\n");
    out.push_str("index b2c3d4e..f5g6h7i 100644\n");
    out.push_str("--- a/src/api/users.ts\n");
    out.push_str("+++ b/src/api/users.ts\n");
    out.push_str(&hunk(1, 1,
        &["import { Router } from 'express';",
          "import { UserService } from '../services/user';",
          "import { db } from '../db';"],
        &[("", "import { validateRequest } from '../middleware/validate';"),
          ("", "import { userSchema, updateUserSchema } from '../schemas/user';")],
        &["", "const router = Router();", "const userService = new UserService();"],
    ));
    out.push_str(&hunk(18, 22,
        &["router.get('/:id', async (req, res) => {",
          "  try {",
          "    const user = await userService.findById(req.params.id);"],
        &[("    res.json(user);",
           "    if (!user) { return res.status(404).json({ error: 'User not found' }); }"),
          ("", "    res.json({ data: user });")],
        &["  } catch (error) {",
          "    res.status(500).json({ error: 'Internal server error' });",
          "  }"],
    ));
    out.push_str(&hunk(30, 36,
        &["router.put('/:id', async (req, res) => {",
          "  try {",
          "    const user = await userService.update(req.params.id, req.body);"],
        &[("router.put('/:id', async (req, res) => {",
           "router.put('/:id', validateRequest(updateUserSchema), async (req, res, next) => {"),
          ("    const user = await userService.update(req.params.id, req.body);",
           "    const user = await userService.update(req.params.id, req.body, req.user);")],
        &["    if (!user) { return res.status(404).json({ error: 'User not found' }); }",
          "    res.json({ data: user });",
          "  } catch (error) {"],
    ));
    out.push_str(&hunk(38, 44,
        &["  } catch (error) {",
          "    res.status(500).json({ error: 'Internal server error' });",
          "  }"],
        &[("    res.status(500).json({ error: 'Internal server error' });", "    next(error);")],
        &["  }", "});", ""],
    ));

    // ── file 3: src/services/auth.ts ────────────────────────────────────────
    out.push_str("\ndiff --git a/src/services/auth.ts b/src/services/auth.ts\n");
    out.push_str("index c3d4e5f..g6h7i8j 100644\n");
    out.push_str("--- a/src/services/auth.ts\n");
    out.push_str("+++ b/src/services/auth.ts\n");
    out.push_str(&hunk(1, 1,
        &["import bcrypt from 'bcrypt';",
          "import jwt from 'jsonwebtoken';",
          "import { config } from '../config';"],
        &[("", "import { Redis } from 'ioredis';"),
          ("", "import { redisClient } from '../config/redis';")],
        &["", "export class AuthService {", "  private readonly saltRounds = 12;"],
    ));
    out.push_str(&hunk(22, 26,
        &["  async login(email: string, password: string): Promise<AuthResult> {",
          "    const user = await this.userRepository.findByEmail(email);",
          "    if (!user) {"],
        &[("      throw new Error('Invalid credentials');",
           "      throw new AppError('Invalid email or password', 401);")],
        &["    }", "    const isValid = await bcrypt.compare(password, user.passwordHash);",
          "    if (!isValid) {"],
    ));
    out.push_str(&hunk(28, 32,
        &["    const isValid = await bcrypt.compare(password, user.passwordHash);",
          "    if (!isValid) {",
          "      // wrong password"],
        &[("      throw new Error('Invalid credentials');",
           "      throw new AppError('Invalid email or password', 401);")],
        &["    }", "", "    // issue token"],
    ));
    out.push_str(&hunk(33, 38,
        &["    // issue token", "    // sign and return", ""],
        &[("    const token = jwt.sign({ userId: user.id }, config.jwtSecret, { expiresIn: '24h' });",
           "    const sessions = await redisClient.keys(`session:${user.id}:*`);"),
          ("", "    if (sessions.length > 0) { await redisClient.del(...sessions); }"),
          ("    return { token, user };",
           "    const token = jwt.sign({ userId: user.id, email: user.email }, config.jwtSecret, { expiresIn: '24h' });"),
          ("", "    const refresh = jwt.sign({ userId: user.id }, config.refreshSecret, { expiresIn: '7d' });"),
          ("", "    await redisClient.set(`session:${user.id}:${token}`, '1', 'EX', 86400);"),
          ("", "    return { token, refresh, user };")],
        &["  }", "}"],
    ));

    // ── file 4: src/models/user.ts ──────────────────────────────────────────
    out.push_str("\ndiff --git a/src/models/user.ts b/src/models/user.ts\n");
    out.push_str("index d4e5f6g..h7i8j9k 100644\n");
    out.push_str("--- a/src/models/user.ts\n");
    out.push_str("+++ b/src/models/user.ts\n");
    out.push_str(&hunk(1, 1,
        &["import { Entity, Column, PrimaryGeneratedColumn } from 'typeorm';",
          "import { IsEmail, IsString, MinLength } from 'class-validator';",
          ""],
        &[("", "import { Exclude } from 'class-transformer';")],
        &["@Entity('users')", "export class User {", "  @PrimaryGeneratedColumn('uuid')"],
    ));
    out.push_str(&hunk(15, 17,
        &["  @Column({ unique: true })",
          "  email: string;",
          ""],
        &[("  @Column()", "  @Column()"),
          ("  password: string;", "  @Exclude()"),
          ("", "  password: string;")],
        &["", "  @Column({ nullable: true })", "  refreshToken: string | null;"],
    ));
    out.push_str(&hunk(25, 28,
        &["  @Column({ default: false })",
          "  isEmailVerified: boolean;",
          ""],
        &[("", "  @Column({ type: 'timestamp', nullable: true })"),
          ("", "  lastLoginAt: Date | null;"),
          ("", "")],
        &["  @Column({ type: 'timestamp', default: () => 'CURRENT_TIMESTAMP' })",
          "  createdAt: Date;", ""],
    ));

    // ── file 5: src/config/database.ts ──────────────────────────────────────
    out.push_str("\ndiff --git a/src/config/database.ts b/src/config/database.ts\n");
    out.push_str("index e5f6g7h..i8j9k0l 100644\n");
    out.push_str("--- a/src/config/database.ts\n");
    out.push_str("+++ b/src/config/database.ts\n");
    out.push_str(&hunk(1, 1,
        &["import { DataSource } from 'typeorm';",
          "import { User } from '../models/user';",
          "import { Post } from '../models/post';"],
        &[("", "import { AuditLog } from '../models/audit-log';"),
          ("", "import { Session } from '../models/session';")],
        &["", "export const AppDataSource = new DataSource({", "  type: 'postgres',"],
    ));
    out.push_str(&hunk(10, 12,
        &["  entities: [User, Post],",
          "  synchronize: false,",
          "  logging: false,"],
        &[("  entities: [User, Post],", "  entities: [User, Post, AuditLog, Session],"),
          ("  logging: false,", "  logging: process.env.DB_LOGGING === 'true',")],
        &["  migrations: ['src/migrations/*.ts'],",
          "  subscribers: [],", "});"],
    ));

    out
}

/// `git push` — realistic object-counting noise.
fn git_push() -> String {
    concat!(
        "Enumerating objects: 147, done.\n",
        "Counting objects: 100% (147/147), done.\n",
        "Delta compression using up to 10 threads\n",
        "Compressing objects: 100% (89/89), done.\n",
        "Writing objects: 100% (98/98), 124.37 KiB | 4.16 MiB/s, done.\n",
        "Total 98 (delta 52), reused 0 (delta 0), pack-reused 0\n",
        "remote: Resolving deltas: 100% (52/52), completed with 31 local objects.\n",
        "To github.com:company/myapp.git\n",
        "   a1b2c3d..e4f5g6h  feature/user-auth -> feature/user-auth\n",
        "Branch 'feature/user-auth' set up to track remote branch 'feature/user-auth' from 'origin'.\n",
    ).to_string()
}

/// `ls -la` on a realistic large project root.
fn ls_project() -> String {
    concat!(
        "total 892\n",
        "drwxr-xr-x  28 user staff   896 Mar 18 14:32 .\n",
        "drwxr-xr-x  15 user staff   480 Mar 15 09:12 ..\n",
        "drwxr-xr-x  12 user staff   384 Mar 18 14:32 .git\n",
        "drwxr-xr-x   4 user staff   128 Mar 10 11:45 .github\n",
        "-rw-r--r--   1 user staff   543 Mar  5 16:20 .gitignore\n",
        "-rw-r--r--   1 user staff   892 Mar 12 10:30 .eslintrc.json\n",
        "-rw-r--r--   1 user staff   234 Mar  8 09:15 .prettierrc\n",
        "-rw-r--r--   1 user staff   128 Mar  5 16:20 .env.example\n",
        "drwxr-xr-x   4 user staff   128 Mar 18 14:00 .next\n",
        "drwxr-xr-x   3 user staff    96 Mar  5 16:20 .vscode\n",
        "-rw-r--r--   1 user staff  2341 Mar 15 11:20 Dockerfile\n",
        "-rw-r--r--   1 user staff   789 Mar 15 11:20 docker-compose.yml\n",
        "-rw-r--r--   1 user staff  1456 Mar 18 14:32 jest.config.ts\n",
        "-rw-r--r--   1 user staff  4521 Mar 16 10:45 package.json\n",
        "-rw-r--r--   1 user staff 89432 Mar 18 14:30 package-lock.json\n",
        "drwxr-xr-x 892 user staff 28544 Mar 18 14:30 node_modules\n",
        "-rw-r--r--   1 user staff   345 Mar 10 14:20 next.config.js\n",
        "-rw-r--r--   1 user staff  1234 Mar 14 09:30 tsconfig.json\n",
        "-rw-r--r--   1 user staff   567 Mar  5 16:20 README.md\n",
        "-rw-r--r--   1 user staff  2345 Mar 12 14:15 CONTRIBUTING.md\n",
        "drwxr-xr-x  18 user staff   576 Mar 18 14:32 src\n",
        "drwxr-xr-x   8 user staff   256 Mar 16 11:00 tests\n",
        "drwxr-xr-x   6 user staff   192 Mar 15 09:00 docs\n",
        "drwxr-xr-x   4 user staff   128 Mar 14 16:30 scripts\n",
        "drwxr-xr-x   3 user staff    96 Mar 12 10:00 migrations\n",
        "drwxr-xr-x   2 user staff    64 Mar 18 14:30 dist\n",
        "drwxr-xr-x   2 user staff    64 Mar 18 14:30 .cache\n",
        "-rw-r--r--   1 user staff   789 Mar 10 11:00 turbo.json\n",
    ).to_string()
}

/// `tsc` — 15 errors across 5 files in the compact `file(line,col): error TSxxxx: message` format
/// that tsc emits by default (no `--pretty` flag, which is common in CI and script invocations).
/// The verbose multi-line form adds code-snippet lines that don't match the handler regex and
/// pass through unchanged; using the standard format tests the file-grouping compression.
fn tsc_errors() -> String {
    // Each error is in the compact single-line format tsc uses by default.
    // We pad with realistic preamble and summary lines that the handler strips.
    let mut out = String::new();
    // Preamble that tsc sometimes emits
    out.push_str("error TS5023: Unknown compiler option 'moduleResolution'.\n");
    out.push_str("\n");

    let errors = [
        ("src/api/users.ts",           42,  "TS2345", "Argument of type 'string | undefined' is not assignable to parameter of type 'string'. Type 'undefined' is not assignable to type 'string'"),
        ("src/api/users.ts",           87,  "TS2339", "Property 'userId' does not exist on type 'Request'. Did you mean 'user'?"),
        ("src/api/users.ts",          134,  "TS2322", "Type 'null' is not assignable to type 'User'"),
        ("src/api/users.ts",          201,  "TS2345", "Argument of type 'number | undefined' is not assignable to parameter of type 'string'"),
        ("src/api/users.ts",          245,  "TS7006", "Parameter 'next' implicitly has an 'any' type"),
        ("src/auth/middleware.ts",     23,   "TS7006", "Parameter 'req' implicitly has an 'any' type"),
        ("src/auth/middleware.ts",     45,   "TS7006", "Parameter 'res' implicitly has an 'any' type"),
        ("src/auth/middleware.ts",     67,   "TS2304", "Cannot find name 'NextFunction'"),
        ("src/auth/middleware.ts",     89,   "TS2345", "Argument of type 'JwtPayload | string' is not assignable to parameter of type 'AuthUser'"),
        ("src/models/user.ts",         15,   "TS1005", "',' expected"),
        ("src/models/user.ts",         89,   "TS2345", "Argument of type 'number' is not assignable to parameter of type 'string'"),
        ("src/components/UserCard.tsx",28,   "TS2741", "Property 'onClick' is missing in type '{}' but required in type 'ButtonProps'"),
        ("src/components/UserCard.tsx",56,   "TS2322", "Type 'string | null' is not assignable to type 'string'"),
        ("src/components/UserCard.tsx",103,  "TS2339", "Property 'loading' does not exist on type 'UserCardProps'"),
        ("src/pages/Profile.tsx",      34,   "TS2304", "Cannot find name 'useParams'"),
        ("src/pages/Profile.tsx",      78,   "TS2345", "Argument of type 'string | undefined' is not assignable to parameter of type 'string'"),
        ("src/pages/Profile.tsx",      112,  "TS2339", "Property 'id' does not exist on type 'never'"),
        ("src/utils/api.ts",           19,   "TS7006", "Parameter 'config' implicitly has an 'any' type"),
        ("src/utils/api.ts",           55,   "TS2322", "Type 'unknown' is not assignable to type 'ApiResponse'"),
        ("src/utils/api.ts",           88,   "TS2345", "Argument of type 'AxiosError' is not assignable to parameter of type 'ApiError'"),
    ];
    for (file, line, code, msg) in &errors {
        out.push_str(&format!("{}({},5): error {}: {}\n", file, line, code, msg));
    }
    out.push_str("\n");
    out.push_str("Found 20 errors in 5 files.\n");
    out.push_str("\n");
    out.push_str("Errors  Files\n");
    out.push_str("     5  src/api/users.ts\n");
    out.push_str("     4  src/auth/middleware.ts\n");
    out.push_str("     2  src/models/user.ts\n");
    out.push_str("     3  src/components/UserCard.tsx\n");
    out.push_str("     3  src/pages/Profile.tsx\n");
    out.push_str("     3  src/utils/api.ts\n");
    out
}

/// `jest` — 10 test suites (2 failing), 150 tests total, 2 failures.
fn jest_output() -> String {
    let mut out = String::new();
    out.push_str(" PASS  tests/utils/format.test.ts (1.234 s)\n");
    out.push_str(" PASS  tests/utils/validate.test.ts (0.892 s)\n");
    out.push_str(" PASS  tests/models/user.test.ts (2.156 s)\n");
    out.push_str(" FAIL  tests/auth/jwt.test.ts (3.421 s)\n");
    out.push_str(" PASS  tests/api/health.test.ts (0.567 s)\n");
    out.push_str(" PASS  tests/api/users.test.ts (4.123 s)\n");
    out.push_str(" FAIL  tests/components/UserCard.test.tsx (2.789 s)\n");
    out.push_str(" PASS  tests/pages/Profile.test.tsx (1.456 s)\n");
    out.push_str(" PASS  tests/hooks/useAuth.test.ts (0.723 s)\n");
    out.push_str(" PASS  tests/services/auth.test.ts (3.234 s)\n");
    out.push_str("\n  ● auth/jwt › should reject expired tokens\n\n");
    out.push_str("    expect(received).toBe(expected)\n\n");
    out.push_str("    Expected: false\n");
    out.push_str("    Received: true\n\n");
    out.push_str("      at Object.<anonymous> (tests/auth/jwt.test.ts:47:5)\n");
    out.push_str("      at Promise.resolve.then (node_modules/jest-jasmine2/build/queueRunner.js:45:12)\n\n");
    out.push_str("  ● components/UserCard › renders user avatar correctly\n\n");
    out.push_str("    TestingLibraryElementError: Unable to find an accessible element with role \"img\"\n\n");
    out.push_str("      at getByRole (node_modules/@testing-library/dom/dist/queries/role.js:108:19)\n");
    out.push_str("      at Object.<anonymous> (tests/components/UserCard.test.tsx:34:26)\n\n");
    out.push_str("Test Suites: 2 failed, 8 passed, 10 total\n");
    out.push_str("Tests:       2 failed, 148 passed, 150 total\n");
    out.push_str("Snapshots:   0 total\n");
    out.push_str("Time:        20.595 s\n");
    out.push_str("Ran all test suites.\n");
    out
}

// ─── benchmark runner ────────────────────────────────────────────────────────

#[test]
fn benchmark_handlers() {
    let git   = GitHandler;
    let cargo = CargoHandler;
    let tsc   = TscHandler;
    let ls    = LsHandler;
    let jest  = JestHandler;

    let (cargo_baseline, cargo_json) = cargo_build();
    let cargo_test_raw               = cargo_test();
    let (status_baseline, porcelain) = git_status();
    let (log_baseline, log_oneline)  = git_log();
    let diff_raw                     = git_diff();
    let push_raw                     = git_push();
    let ls_raw                       = ls_project();
    let tsc_raw                      = tsc_errors();
    let jest_raw                     = jest_output();

    struct Row { op: &'static str, in_tok: usize, out_tok: usize }

    let rows: Vec<Row> = vec![
        {
            // cargo build: baseline = human-readable, filter receives JSON
            let out = run(&cargo, &cargo_json, &["cargo", "build"]);
            Row { op: "cargo build  (130 deps, 5 warnings)",
                  in_tok:  count_tokens(&cargo_baseline),
                  out_tok: count_tokens(&out) }
        },
        {
            let out = run(&cargo, &cargo_test_raw, &["cargo", "test"]);
            Row { op: "cargo test   (200 tests, 2 failures)",
                  in_tok:  count_tokens(&cargo_test_raw),
                  out_tok: count_tokens(&out) }
        },
        {
            // git status: baseline = verbose, filter receives porcelain
            let out = run(&git, &porcelain, &["git", "status"]);
            Row { op: "git status   (10 staged, 40 modified, 8 untracked)",
                  in_tok:  count_tokens(&status_baseline),
                  out_tok: count_tokens(&out) }
        },
        {
            // git log: baseline = full verbose, filter receives --oneline
            let out = run(&git, &log_oneline, &["git", "log"]);
            Row { op: "git log      (25 commits)",
                  in_tok:  count_tokens(&log_baseline),
                  out_tok: count_tokens(&out) }
        },
        {
            let out = run(&git, &diff_raw, &["git", "diff"]);
            Row { op: "git diff     (3 files, ~60 lines changed)",
                  in_tok:  count_tokens(&diff_raw),
                  out_tok: count_tokens(&out) }
        },
        {
            let out = run(&git, &push_raw, &["git", "push"]);
            Row { op: "git push     (object-counting noise)",
                  in_tok:  count_tokens(&push_raw),
                  out_tok: count_tokens(&out) }
        },
        {
            let out = run(&ls, &ls_raw, &["ls"]);
            Row { op: "ls           (project root, 28 entries)",
                  in_tok:  count_tokens(&ls_raw),
                  out_tok: count_tokens(&out) }
        },
        {
            let out = run(&tsc, &tsc_raw, &["tsc"]);
            Row { op: "tsc          (20 errors, 5 files)",
                  in_tok:  count_tokens(&tsc_raw),
                  out_tok: count_tokens(&out) }
        },
        {
            let out = run(&jest, &jest_raw, &["jest"]);
            Row { op: "jest         (150 tests, 2 failures)",
                  in_tok:  count_tokens(&jest_raw),
                  out_tok: count_tokens(&out) }
        },
    ];

    println!();
    println!("{:<52} {:>12} {:>10} {:>10}", "Operation", "Without CCR", "With CCR", "Savings");
    println!("{}", "─".repeat(88));

    let mut total_in  = 0usize;
    let mut total_out = 0usize;

    for row in &rows {
        let pct = savings_pct(row.in_tok, row.out_tok);
        println!("{:<52} {:>12} {:>10} {:>9.0}%",
            row.op, row.in_tok, row.out_tok, pct);
        total_in  += row.in_tok;
        total_out += row.out_tok;
    }

    println!("{}", "─".repeat(88));
    let total_pct = savings_pct(total_in, total_out);
    println!("{:<52} {:>12} {:>10} {:>9.0}%", "TOTAL", total_in, total_out, total_pct);
    println!();

    // Sanity assertions — each handler must reduce tokens by at least 10%
    for row in &rows {
        let pct = savings_pct(row.in_tok, row.out_tok);
        assert!(
            pct >= 10.0,
            "Handler for '{}' saved only {:.0}% — expected ≥10%",
            row.op, pct
        );
    }
}
