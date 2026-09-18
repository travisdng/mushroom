// Credential detection rules, vendored from gitleaks.
//
// GENERATED FILE -- do not edit by hand.
// Regenerate with: python tools/vendor-gitleaks-rules.py <gitleaks.toml>
//
// Source:  https://github.com/gitleaks/gitleaks
// Version: v8.30.1
// Rules:   220 vendored, 1 dropped (generic-api-key)
//
// gitleaks is MIT licensed:
//
//   Copyright (c) 2019 Zachary Rice
//
//   Permission is hereby granted, free of charge, to any person obtaining a
//   copy of this software and associated documentation files (the "Software"),
//   to deal in the Software without restriction, including without limitation
//   the rights to use, copy, modify, merge, publish, distribute, sublicense,
//   and/or sell copies of the Software, and to permit persons to whom the
//   Software is furnished to do so, subject to the following conditions:
//
//   The above copyright notice and this permission notice shall be included in
//   all copies or substantial portions of the Software.
//
//   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
//   IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
//   FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
//   AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
//   LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
//   FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER
//   DEALINGS IN THE SOFTWARE.
//
// The patterns are unmodified except where a `Bound widened for Rust` comment
// says otherwise, and those are listed in the generator. What changes here is
// the shape they are stored in and which ones are included: a rule tuned for
// scanning a source tree is not automatically right for scanning prose.

use super::rules::{Confidence, Rule};

/// Provider credential patterns, one per shape that does not occur by accident.
pub const GITLEAKS_RULES: &[Rule] = &[
    Rule {
        name: "1password-secret-key",
        pattern: r#"\bA3-[A-Z0-9]{6}-(?:(?:[A-Z0-9]{11})|(?:[A-Z0-9]{6}-[A-Z0-9]{5}))-[A-Z0-9]{5}-[A-Z0-9]{5}-[A-Z0-9]{5}\b"#,
        confidence: Confidence::Entropy(3.8),
        keywords: &["a3-"],
    },
    Rule {
        name: "1password-service-account-token",
        pattern: r#"ops_eyJ[a-zA-Z0-9+/]{250,}={0,3}"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["ops_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "adafruit-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:adafruit)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9_-]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["adafruit"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "adobe-client-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:adobe)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["adobe"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "adobe-client-secret",
        pattern: r#"\b(p8e-(?i)[a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["p8e-"],
    },
    Rule {
        name: "age-secret-key",
        pattern: r#"AGE-SECRET-KEY-1[QPZRY9X8GF2TVDW0S3JN54KHCE6MUA7L]{58}"#,
        confidence: Confidence::Shape,
        keywords: &["age-secret-key-1"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "airtable-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:airtable)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{17})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["airtable"],
    },
    Rule {
        name: "airtable-personnal-access-token",
        pattern: r#"\b(pat[[:alnum:]]{14}\.[a-f0-9]{64})\b"#,
        confidence: Confidence::Shape,
        keywords: &["airtable"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "algolia-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:algolia)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["algolia"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "alibaba-access-key-id",
        pattern: r#"\b(LTAI(?i)[a-z0-9]{20})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["ltai"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "alibaba-secret-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:alibaba)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{30})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["alibaba"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "anthropic-admin-api-key",
        pattern: r#"\b(sk-ant-admin01-[a-zA-Z0-9_\-]{93}AA)(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["sk-ant-admin01"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "anthropic-api-key",
        pattern: r#"\b(sk-ant-api03-[a-zA-Z0-9_\-]{93}AA)(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["sk-ant-api03"],
    },
    Rule {
        name: "artifactory-api-key",
        pattern: r#"\bAKCp[A-Za-z0-9]{69}\b"#,
        confidence: Confidence::Entropy(4.5),
        keywords: &["akcp"],
    },
    Rule {
        name: "artifactory-reference-token",
        pattern: r#"\bcmVmd[A-Za-z0-9]{59}\b"#,
        confidence: Confidence::Entropy(4.5),
        keywords: &["cmvmd"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "asana-client-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:asana)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9]{16})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["asana"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "asana-client-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:asana)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["asana"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "atlassian-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:(?-i:ATLASSIAN|[Aa]tlassian)|(?-i:CONFLUENCE|[Cc]onfluence)|(?-i:JIRA|[Jj]ira))(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{20}[a-f0-9]{4})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)|\b(ATATT3[A-Za-z0-9_\-=]{186})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.5),
        keywords: &["atlassian", "confluence", "jira", "atatt3"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "authress-service-client-access-key",
        pattern: r#"\b((?:sc|ext|scauth|authress)_(?i)[a-z0-9]{5,30}\.[a-z0-9]{4,6}\.(?-i:acc)[_-][a-z0-9-]{10,32}\.[a-z0-9+/_=-]{30,120})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["sc_", "ext_", "scauth_", "authress_"],
    },
    Rule {
        name: "aws-access-token",
        pattern: r#"\b((?:A3T[A-Z0-9]|AKIA|ASIA|ABIA|ACCA)[A-Z2-7]{16})\b"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["a3t", "akia", "asia", "abia", "acca"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "aws-amazon-bedrock-api-key-long-lived",
        pattern: r#"\b(ABSK[A-Za-z0-9+/]{109,269}={0,2})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["absk"],
    },
    Rule {
        name: "aws-amazon-bedrock-api-key-short-lived",
        pattern: r#"bedrock-api-key-YmVkcm9jay5hbWF6b25hd3MuY29t"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["bedrock-api-key-"],
    },
    Rule {
        name: "azure-ad-client-secret",
        pattern: r#"(?:^|[\\'"\x60\s>=:(,)])([a-zA-Z0-9_~.]{3}\dQ~[a-zA-Z0-9_~.-]{31,34})(?:$|[\\'"\x60\s<),])"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["q~"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "beamer-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:beamer)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(b_[a-z0-9=_\-]{44})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["beamer"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "bitbucket-client-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:bitbucket)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["bitbucket"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "bitbucket-client-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:bitbucket)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9=_\-]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["bitbucket"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "bittrex-access-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:bittrex)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["bittrex"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "bittrex-secret-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:bittrex)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["bittrex"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "cisco-meraki-api-key",
        pattern: r#"[\w.-]{0,50}?(?i:[\w.-]{0,50}?(?:(?-i:[Mm]eraki|MERAKI))(?:[ \t\w.-]{0,20})[\s'"]{0,3})(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9a-f]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["meraki"],
    },
    Rule {
        name: "clickhouse-cloud-api-secret-key",
        pattern: r#"\b(4b1d[A-Za-z0-9]{38})\b"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["4b1d"],
    },
    Rule {
        name: "clojars-api-token",
        pattern: r#"(?i)CLOJARS_[a-z0-9]{60}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["clojars_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "cloudflare-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:cloudflare)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9_-]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["cloudflare"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "cloudflare-global-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:cloudflare)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{37})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["cloudflare"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "cloudflare-origin-ca-key",
        pattern: r#"\b(v1\.0-[a-f0-9]{24}-[a-f0-9]{146})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["cloudflare", "v1.0-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "codecov-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:codecov)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["codecov"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "cohere-api-token",
        pattern: r#"[\w.-]{0,50}?(?i:[\w.-]{0,50}?(?:cohere|CO_API_KEY)(?:[ \t\w.-]{0,20})[\s'"]{0,3})(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-zA-Z0-9]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["cohere", "co_api_key"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "coinbase-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:coinbase)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9_-]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["coinbase"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "confluent-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:confluent)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{16})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["confluent"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "confluent-secret-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:confluent)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["confluent"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "contentful-delivery-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:contentful)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9=_\-]{43})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["contentful"],
    },
    Rule {
        name: "curl-auth-header",
        pattern: r#"\bcurl\b(?:.*?|.*?(?:[\r\n]{1,2}.*?){1,5})[ \t\n\r](?:-H|--header)(?:=|[ \t]{0,5})(?:"(?i)(?:Authorization:[ \t]{0,5}(?:Basic[ \t]([a-z0-9+/]{8,}={0,3})|(?:Bearer|(?:Api-)?Token)[ \t]([\w=~@.+/-]{8,})|([\w=~@.+/-]{8,}))|(?:(?:X-(?:[a-z]+-)?)?(?:Api-?)?(?:Key|Token)):[ \t]{0,5}([\w=~@.+/-]{8,}))"|'(?i)(?:Authorization:[ \t]{0,5}(?:Basic[ \t]([a-z0-9+/]{8,}={0,3})|(?:Bearer|(?:Api-)?Token)[ \t]([\w=~@.+/-]{8,})|([\w=~@.+/-]{8,}))|(?:(?:X-(?:[a-z]+-)?)?(?:Api-?)?(?:Key|Token)):[ \t]{0,5}([\w=~@.+/-]{8,}))')(?:\B|\s|\z)"#,
        confidence: Confidence::Entropy(2.75),
        keywords: &["curl"],
    },
    Rule {
        name: "curl-auth-user",
        pattern: r#"\bcurl\b(?:.*|.*(?:[\r\n]{1,2}.*){1,5})[ \t\n\r](?:-u|--user)(?:=|[ \t]{0,5})("(:[^"]{3,}|[^:"]{3,}:|[^:"]{3,}:[^"]{3,})"|'([^:']{3,}:[^']{3,})'|((?:"[^"]{3,}"|'[^']{3,}'|[\w$@.-]+):(?:"[^"]{3,}"|'[^']{3,}'|[\w${}@.-]+)))(?:\s|\z)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["curl"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "databricks-api-token",
        pattern: r#"\b(dapi[a-f0-9]{32}(?:-\d)?)(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["dapi"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "datadog-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:datadog)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["datadog"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "defined-networking-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:dnkey)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(dnkey-[a-z0-9=_\-]{26}-[a-z0-9=_\-]{52})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["dnkey"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "digitalocean-access-token",
        pattern: r#"\b(doo_v1_[a-f0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["doo_v1_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "digitalocean-pat",
        pattern: r#"\b(dop_v1_[a-f0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["dop_v1_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "digitalocean-refresh-token",
        pattern: r#"(?i)\b(dor_v1_[a-f0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["dor_v1_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "discord-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:discord)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["discord"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "discord-client-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:discord)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9]{18})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["discord"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "discord-client-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:discord)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9=_\-]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["discord"],
    },
    Rule {
        name: "doppler-api-token",
        pattern: r#"dp\.pt\.(?i)[a-z0-9]{43}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["dp.pt."],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "droneci-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:droneci)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["droneci"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "dropbox-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:dropbox)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{15})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["dropbox"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "dropbox-long-lived-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:dropbox)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{11}(AAAAAAAAAA)[a-z0-9\-_=]{43})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["dropbox"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "dropbox-short-lived-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:dropbox)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(sl\.[a-z0-9\-=_]{135})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["dropbox"],
    },
    Rule {
        name: "duffel-api-token",
        pattern: r#"duffel_(?:test|live)_(?i)[a-z0-9_\-=]{43}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["duffel_"],
    },
    Rule {
        name: "dynatrace-api-token",
        pattern: r#"dt0c01\.(?i)[a-z0-9]{24}\.[a-z0-9]{64}"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["dt0c01."],
    },
    Rule {
        name: "easypost-api-token",
        pattern: r#"\bEZAK(?i)[a-z0-9]{54}\b"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["ezak"],
    },
    Rule {
        name: "easypost-test-api-token",
        pattern: r#"\bEZTK(?i)[a-z0-9]{54}\b"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["eztk"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "etsy-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:(?-i:ETSY|[Ee]tsy))(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{24})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["etsy"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "facebook-access-token",
        pattern: r#"(?i)\b(\d{15,16}(\||%)[0-9a-z\-_]{27,40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["facebook"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "facebook-page-access-token",
        pattern: r#"\b(EAA[MC](?i)[a-z0-9]{100,})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["eaam", "eaac"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "facebook-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:facebook)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["facebook"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "fastly-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:fastly)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9=_\-]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["fastly"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "finicity-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:finicity)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["finicity"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "finicity-client-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:finicity)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{20})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["finicity"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "finnhub-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:finnhub)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{20})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["finnhub"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "flickr-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:flickr)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["flickr"],
    },
    Rule {
        name: "flutterwave-encryption-key",
        pattern: r#"FLWSECK_TEST-(?i)[a-h0-9]{12}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["flwseck_test"],
    },
    Rule {
        name: "flutterwave-public-key",
        pattern: r#"FLWPUBK_TEST-(?i)[a-h0-9]{32}-X"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["flwpubk_test"],
    },
    Rule {
        name: "flutterwave-secret-key",
        pattern: r#"FLWSECK_TEST-(?i)[a-h0-9]{32}-X"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["flwseck_test"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "flyio-access-token",
        pattern: r#"\b((?:fo1_[\w-]{43}|fm1[ar]_[a-zA-Z0-9+\/]{100,}={0,3}|fm2_[a-zA-Z0-9+\/]{100,}={0,3}))(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["fo1_", "fm1", "fm2_"],
    },
    Rule {
        name: "frameio-api-token",
        pattern: r#"fio-u-(?i)[a-z0-9\-_=]{64}"#,
        confidence: Confidence::Shape,
        keywords: &["fio-u-"],
    },
    Rule {
        name: "freemius-secret-key",
        pattern: r#"(?i)["']secret_key["']\s*=>\s*["'](sk_[\S]{29})["']"#,
        confidence: Confidence::Shape,
        keywords: &["secret_key"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "freshbooks-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:freshbooks)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["freshbooks"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "gcp-api-key",
        pattern: r#"\b(AIza[\w-]{35})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["aiza"],
    },
    Rule {
        name: "github-app-token",
        pattern: r#"(?:ghu|ghs)_[0-9a-zA-Z]{36}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["ghu_", "ghs_"],
    },
    Rule {
        name: "github-fine-grained-pat",
        pattern: r#"github_pat_\w{82}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["github_pat_"],
    },
    Rule {
        name: "github-oauth",
        pattern: r#"gho_[0-9a-zA-Z]{36}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["gho_"],
    },
    Rule {
        name: "github-pat",
        pattern: r#"ghp_[0-9a-zA-Z]{36}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["ghp_"],
    },
    Rule {
        name: "github-refresh-token",
        pattern: r#"ghr_[0-9a-zA-Z]{36}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["ghr_"],
    },
    Rule {
        name: "gitlab-cicd-job-token",
        pattern: r#"glcbt-[0-9a-zA-Z]{1,5}_[0-9a-zA-Z_-]{20}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glcbt-"],
    },
    Rule {
        name: "gitlab-deploy-token",
        pattern: r#"gldt-[0-9a-zA-Z_\-]{20}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["gldt-"],
    },
    Rule {
        name: "gitlab-feature-flag-client-token",
        pattern: r#"glffct-[0-9a-zA-Z_\-]{20}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glffct-"],
    },
    Rule {
        name: "gitlab-feed-token",
        pattern: r#"glft-[0-9a-zA-Z_\-]{20}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glft-"],
    },
    Rule {
        name: "gitlab-incoming-mail-token",
        pattern: r#"glimt-[0-9a-zA-Z_\-]{25}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glimt-"],
    },
    Rule {
        name: "gitlab-kubernetes-agent-token",
        pattern: r#"glagent-[0-9a-zA-Z_\-]{50}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glagent-"],
    },
    Rule {
        name: "gitlab-oauth-app-secret",
        pattern: r#"gloas-[0-9a-zA-Z_\-]{64}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["gloas-"],
    },
    Rule {
        name: "gitlab-pat",
        pattern: r#"glpat-[\w-]{20}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glpat-"],
    },
    Rule {
        name: "gitlab-pat-routable",
        pattern: r#"\bglpat-[0-9a-zA-Z_-]{27,300}\.[0-9a-z]{2}[0-9a-z]{7}\b"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["glpat-"],
    },
    Rule {
        name: "gitlab-ptt",
        pattern: r#"glptt-[0-9a-f]{40}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glptt-"],
    },
    Rule {
        name: "gitlab-rrt",
        pattern: r#"GR1348941[\w-]{20}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["gr1348941"],
    },
    Rule {
        name: "gitlab-runner-authentication-token",
        pattern: r#"glrt-[0-9a-zA-Z_\-]{20}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glrt-"],
    },
    Rule {
        name: "gitlab-runner-authentication-token-routable",
        pattern: r#"\bglrt-t\d_[0-9a-zA-Z_\-]{27,300}\.[0-9a-z]{2}[0-9a-z]{7}\b"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["glrt-"],
    },
    Rule {
        name: "gitlab-scim-token",
        pattern: r#"glsoat-[0-9a-zA-Z_\-]{20}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glsoat-"],
    },
    Rule {
        name: "gitlab-session-cookie",
        pattern: r#"_gitlab_session=[0-9a-z]{32}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["_gitlab_session="],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "gitter-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:gitter)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9_-]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["gitter"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "gocardless-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:gocardless)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(live_(?i)[a-z0-9\-_=]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["live_", "gocardless"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "grafana-api-key",
        pattern: r#"(?i)\b(eyJrIjoi[A-Za-z0-9]{70,400}={0,3})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["eyjrijoi"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "grafana-cloud-api-token",
        pattern: r#"(?i)\b(glc_[A-Za-z0-9+/]{32,400}={0,3})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glc_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "grafana-service-account-token",
        pattern: r#"(?i)\b(glsa_[A-Za-z0-9]{32}_[A-Fa-f0-9]{8})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["glsa_"],
    },
    Rule {
        name: "harness-api-key",
        pattern: r#"(?:pat|sat)\.[a-zA-Z0-9_-]{22}\.[a-zA-Z0-9]{24}\.[a-zA-Z0-9]{20}"#,
        confidence: Confidence::Shape,
        keywords: &["pat.", "sat."],
    },
    Rule {
        name: "hashicorp-tf-api-token",
        pattern: r#"(?i)[a-z0-9]{14}\.(?-i:atlasv1)\.[a-z0-9\-_=]{60,70}"#,
        confidence: Confidence::Entropy(3.5),
        keywords: &["atlasv1"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "hashicorp-tf-password",
        pattern: r#"(?i)[\w.-]{0,50}?(?:administrator_login_password|password)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}("[a-z0-9=_\-]{8,20}")(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["administrator_login_password", "password"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "heroku-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:heroku)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["heroku"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "heroku-api-key-v2",
        pattern: r#"\b((HRKU-AA[0-9a-zA-Z_-]{58}))(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["hrku-aa"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "hubspot-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:hubspot)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9A-F]{8}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{12})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["hubspot"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "huggingface-access-token",
        pattern: r#"\b(hf_(?i:[a-z]{34}))(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["hf_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "huggingface-organization-api-token",
        pattern: r#"\b(api_org_(?i:[a-z]{34}))(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["api_org_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "infracost-api-token",
        pattern: r#"\b(ico-[a-zA-Z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["ico-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "intercom-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:intercom)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9=_\-]{60})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["intercom"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "intra42-client-secret",
        pattern: r#"\b(s-s4t2(?:ud|af)-(?i)[abcdef0123456789]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["intra", "s-s4t2ud-", "s-s4t2af-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "jfrog-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:jfrog|artifactory|bintray|xray)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{73})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["jfrog", "artifactory", "bintray", "xray"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "jfrog-identity-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:jfrog|artifactory|bintray|xray)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["jfrog", "artifactory", "bintray", "xray"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "jwt",
        pattern: r#"\b(ey[a-zA-Z0-9]{17,}\.ey[a-zA-Z0-9\/\\_-]{17,}\.(?:[a-zA-Z0-9\/\\_-]{10,}={0,2})?)(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["ey"],
    },
    Rule {
        name: "jwt-base64",
        pattern: r#"\bZXlK(?:(?P<alg>aGJHY2lPaU)|(?P<apu>aGNIVWlPaU)|(?P<apv>aGNIWWlPaU)|(?P<aud>aGRXUWlPaU)|(?P<b64>aU5qUWlP)|(?P<crit>amNtbDBJanBi)|(?P<cty>amRIa2lPaU)|(?P<epk>bGNHc2lPbn)|(?P<enc>bGJtTWlPaU)|(?P<jku>cWEzVWlPaU)|(?P<jwk>cWQyc2lPb)|(?P<iss>cGMzTWlPaU)|(?P<iv>cGRpSTZJ)|(?P<kid>cmFXUWlP)|(?P<key_ops>clpYbGZiM0J6SWpwY)|(?P<kty>cmRIa2lPaUp)|(?P<nonce>dWIyNWpaU0k2)|(?P<p2c>d01tTWlP)|(?P<p2s>d01uTWlPaU)|(?P<ppt>d2NIUWlPaU)|(?P<sub>emRXSWlPaU)|(?P<svt>emRuUWlP)|(?P<tag>MFlXY2lPaU)|(?P<typ>MGVYQWlPaUp)|(?P<url>MWNtd2l)|(?P<use>MWMyVWlPaUp)|(?P<ver>MlpYSWlPaU)|(?P<version>MlpYSnphVzl1SWpv)|(?P<x>NElqb2)|(?P<x5c>NE5XTWlP)|(?P<x5t>NE5YUWlPaU)|(?P<x5ts256>NE5YUWpVekkxTmlJNkl)|(?P<x5u>NE5YVWlPaU)|(?P<zip>NmFYQWlPaU))[a-zA-Z0-9\/\\_+\-\r\n]{40,}={0,2}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["zxlk"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "kraken-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:kraken)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9\/=_\+\-]{80,90})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["kraken"],
    },
    Rule {
        name: "kubernetes-secret-yaml",
        pattern: r#"(?i)(?:\bkind:[ \t]*["']?\bsecret\b["']?(?s:.){0,200}?\bdata:(?s:.){0,100}?\s+([\w.-]+:(?:[ \t]*(?:\||>[-+]?)\s+)?[ \t]*(?:["']?[a-z0-9+/]{10,}={0,3}["']?|\{\{[ \t\w"|$:=,.-]+}}|""|''))|\bdata:(?s:.){0,100}?\s+([\w.-]+:(?:[ \t]*(?:\||>[-+]?)\s+)?[ \t]*(?:["']?[a-z0-9+/]{10,}={0,3}["']?|\{\{[ \t\w"|$:=,.-]+}}|""|''))(?s:.){0,200}?\bkind:[ \t]*["']?\bsecret\b["']?)"#,
        confidence: Confidence::Shape,
        keywords: &["secret"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "kucoin-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:kucoin)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{24})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["kucoin"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "kucoin-secret-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:kucoin)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["kucoin"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "launchdarkly-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:launchdarkly)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9=_\-]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["launchdarkly"],
    },
    Rule {
        name: "linear-api-key",
        pattern: r#"lin_api_(?i)[a-z0-9]{40}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["lin_api_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "linear-client-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:linear)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["linear"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "linkedin-client-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:linked[_-]?in)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{14})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["linkedin", "linked_in", "linked-in"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "linkedin-client-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:linked[_-]?in)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{16})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["linkedin", "linked_in", "linked-in"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "lob-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:lob)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}((live|test)_[a-f0-9]{35})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["test_", "live_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "lob-pub-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:lob)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}((test|live)_pub_[a-f0-9]{31})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["test_pub", "live_pub", "_pub"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "looker-client-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:looker)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{20})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["looker"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "looker-client-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:looker)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{24})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["looker"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "mailchimp-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:MailchimpSDK.initialize|mailchimp)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{32}-us\d\d)(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["mailchimp"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "mailgun-private-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:mailgun)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(key-[a-f0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["mailgun"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "mailgun-pub-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:mailgun)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(pubkey-[a-f0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["mailgun"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "mailgun-signing-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:mailgun)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-h0-9]{32}-[a-h0-9]{8}-[a-h0-9]{8})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["mailgun"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "mapbox-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:mapbox)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(pk\.[a-z0-9]{60}\.[a-z0-9]{22})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["mapbox"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "mattermost-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:mattermost)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{26})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["mattermost"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "maxmind-license-key",
        pattern: r#"\b([A-Za-z0-9]{6}_[A-Za-z0-9]{29}_mmk)(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["_mmk"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "messagebird-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:message[_-]?bird)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{25})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["messagebird", "message-bird", "message_bird"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "messagebird-client-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:message[_-]?bird)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["messagebird", "message-bird", "message_bird"],
    },
    Rule {
        name: "microsoft-teams-webhook",
        pattern: r#"https://[a-z0-9]+\.webhook\.office\.com/webhookb2/[a-z0-9]{8}-([a-z0-9]{4}-){3}[a-z0-9]{12}@[a-z0-9]{8}-([a-z0-9]{4}-){3}[a-z0-9]{12}/IncomingWebhook/[a-z0-9]{32}/[a-z0-9]{8}-([a-z0-9]{4}-){3}[a-z0-9]{12}"#,
        confidence: Confidence::Shape,
        keywords: &["webhook.office.com", "webhookb2", "incomingwebhook"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "netlify-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:netlify)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9=_\-]{40,46})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["netlify"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "new-relic-browser-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:new-relic|newrelic|new_relic)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(NRJS-[a-f0-9]{19})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["nrjs-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "new-relic-insert-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:new-relic|newrelic|new_relic)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(NRII-[a-z0-9-]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["nrii-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "new-relic-user-api-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:new-relic|newrelic|new_relic)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["new-relic", "newrelic", "new_relic"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "new-relic-user-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:new-relic|newrelic|new_relic)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(NRAK-[a-z0-9]{27})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["nrak"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "notion-api-token",
        pattern: r#"\b(ntn_[0-9]{11}[A-Za-z0-9]{32}[A-Za-z0-9]{3})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["ntn_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "npm-access-token",
        pattern: r#"(?i)\b(npm_[a-z0-9]{36})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["npm_"],
    },
    Rule {
        name: "nuget-config-password",
        pattern: r#"(?i)<add key=\"(?:(?:ClearText)?Password)\"\s*value=\"(.{8,})\"\s*/>"#,
        confidence: Confidence::Entropy(1.0),
        keywords: &["<add key="],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "nytimes-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:nytimes|new-york-times,|newyorktimes)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9=_\-]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["nytimes", "new-york-times", "newyorktimes"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "octopus-deploy-api-key",
        pattern: r#"\b(API-[A-Z0-9]{26})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["api-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "okta-access-token",
        pattern: r#"[\w.-]{0,50}?(?i:[\w.-]{0,50}?(?:(?-i:[Oo]kta|OKTA))(?:[ \t\w.-]{0,20})[\s'"]{0,3})(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(00[\w=\-]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["okta"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "openai-api-key",
        pattern: r#"\b(sk-(?:proj|svcacct|admin)-(?:[A-Za-z0-9_-]{74}|[A-Za-z0-9_-]{58})T3BlbkFJ(?:[A-Za-z0-9_-]{74}|[A-Za-z0-9_-]{58})\b|sk-[a-zA-Z0-9]{20}T3BlbkFJ[a-zA-Z0-9]{20})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["t3blbkfj"],
    },
    Rule {
        name: "openshift-user-token",
        pattern: r#"\b(sha256~[\w-]{43})(?:[^\w-]|\z)"#,
        confidence: Confidence::Entropy(3.5),
        keywords: &["sha256~"],
    },
    Rule {
        name: "perplexity-api-key",
        pattern: r#"\b(pplx-[a-zA-Z0-9]{48})(?:[\x60'"\s;]|\\[nr]|$|\b)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["pplx-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "plaid-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:plaid)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(access-(?:sandbox|development|production)-[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["plaid"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "plaid-client-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:plaid)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{24})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.5),
        keywords: &["plaid"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "plaid-secret-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:plaid)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{30})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.5),
        keywords: &["plaid"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "planetscale-api-token",
        pattern: r#"\b(pscale_tkn_(?i)[\w=\.-]{32,64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["pscale_tkn_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "planetscale-oauth-token",
        pattern: r#"\b(pscale_oauth_[\w=\.-]{32,64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["pscale_oauth_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "planetscale-password",
        pattern: r#"(?i)\b(pscale_pw_(?i)[\w=\.-]{32,64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["pscale_pw_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "postman-api-token",
        pattern: r#"\b(PMAK-(?i)[a-f0-9]{24}\-[a-f0-9]{34})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["pmak-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "prefect-api-token",
        pattern: r#"\b(pnu_[a-zA-Z0-9]{36})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["pnu_"],
    },
    Rule {
        name: "private-key",
        pattern: r#"(?i)-----BEGIN[ A-Z0-9_-]{0,100}PRIVATE KEY(?: BLOCK)?-----[\s\S-]{64,}?KEY(?: BLOCK)?-----"#,
        confidence: Confidence::Shape,
        keywords: &["-----begin"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "privateai-api-token",
        pattern: r#"[\w.-]{0,50}?(?i:[\w.-]{0,50}?(?:private[_-]?ai)(?:[ \t\w.-]{0,20})[\s'"]{0,3})(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{32})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["privateai", "private_ai", "private-ai"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "pulumi-api-token",
        pattern: r#"\b(pul-[a-f0-9]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["pul-"],
    },
    Rule {
        // Bound widened for Rust: {50,1000} -> {50,}
        name: "pypi-upload-token",
        pattern: r#"pypi-AgEIcHlwaS5vcmc[\w-]{50,}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["pypi-ageichlwas5vcmc"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "rapidapi-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:rapidapi)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9_-]{50})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["rapidapi"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "readme-api-token",
        pattern: r#"\b(rdme_[a-z0-9]{70})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["rdme_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "rubygems-api-token",
        pattern: r#"\b(rubygems_[a-f0-9]{48})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["rubygems_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "scalingo-api-token",
        pattern: r#"\b(tk-us-[\w-]{48})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["tk-us-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sendbird-access-id",
        pattern: r#"(?i)[\w.-]{0,50}?(?:sendbird)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["sendbird"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sendbird-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:sendbird)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["sendbird"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sendgrid-api-token",
        pattern: r#"\b(SG\.(?i)[a-z0-9=_\-\.]{66})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["sg."],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sendinblue-api-token",
        pattern: r#"\b(xkeysib-[a-f0-9]{64}\-(?i)[a-z0-9]{16})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["xkeysib-"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sentry-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:sentry)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["sentry"],
    },
    Rule {
        name: "sentry-org-token",
        pattern: r#"\bsntrys_eyJpYXQiO[a-zA-Z0-9+/]{10,200}(?:LCJyZWdpb25fdXJs|InJlZ2lvbl91cmwi|cmVnaW9uX3VybCI6)[a-zA-Z0-9+/]{10,200}={0,2}_[a-zA-Z0-9+/]{43}(?:[^a-zA-Z0-9+/]|\z)"#,
        confidence: Confidence::Entropy(4.5),
        keywords: &["sntrys_eyjpyxqio"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sentry-user-token",
        pattern: r#"\b(sntryu_[a-f0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.5),
        keywords: &["sntryu_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "settlemint-application-access-token",
        pattern: r#"\b(sm_aat_[a-zA-Z0-9]{16})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["sm_aat"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "settlemint-personal-access-token",
        pattern: r#"\b(sm_pat_[a-zA-Z0-9]{16})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["sm_pat"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "settlemint-service-access-token",
        pattern: r#"\b(sm_sat_[a-zA-Z0-9]{16})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["sm_sat"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "shippo-api-token",
        pattern: r#"\b(shippo_(?:live|test)_[a-fA-F0-9]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["shippo_"],
    },
    Rule {
        name: "shopify-access-token",
        pattern: r#"shpat_[a-fA-F0-9]{32}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["shpat_"],
    },
    Rule {
        name: "shopify-custom-access-token",
        pattern: r#"shpca_[a-fA-F0-9]{32}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["shpca_"],
    },
    Rule {
        name: "shopify-private-app-access-token",
        pattern: r#"shppa_[a-fA-F0-9]{32}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["shppa_"],
    },
    Rule {
        name: "shopify-shared-secret",
        pattern: r#"shpss_[a-fA-F0-9]{32}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["shpss_"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sidekiq-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:BUNDLE_ENTERPRISE__CONTRIBSYS__COM|BUNDLE_GEMS__CONTRIBSYS__COM)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-f0-9]{8}:[a-f0-9]{8})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &[
            "bundle_enterprise__contribsys__com",
            "bundle_gems__contribsys__com",
        ],
    },
    Rule {
        name: "sidekiq-sensitive-url",
        pattern: r#"(?i)\bhttps?://([a-f0-9]{8}:[a-f0-9]{8})@(?:gems.contribsys.com|enterprise.contribsys.com)(?:[\/|\#|\?|:]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["gems.contribsys.com", "enterprise.contribsys.com"],
    },
    Rule {
        name: "slack-app-token",
        pattern: r#"(?i)xapp-\d-[A-Z0-9]+-\d+-[a-z0-9]+"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["xapp"],
    },
    Rule {
        name: "slack-bot-token",
        pattern: r#"xoxb-[0-9]{10,13}-[0-9]{10,13}[a-zA-Z0-9-]*"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["xoxb"],
    },
    Rule {
        name: "slack-config-access-token",
        pattern: r#"(?i)xoxe.xox[bp]-\d-[A-Z0-9]{163,166}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["xoxe.xoxb-", "xoxe.xoxp-"],
    },
    Rule {
        name: "slack-config-refresh-token",
        pattern: r#"(?i)xoxe-\d-[A-Z0-9]{146}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["xoxe-"],
    },
    Rule {
        name: "slack-legacy-bot-token",
        pattern: r#"xoxb-[0-9]{8,14}-[a-zA-Z0-9]{18,26}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["xoxb"],
    },
    Rule {
        name: "slack-legacy-token",
        pattern: r#"xox[os]-\d+-\d+-\d+-[a-fA-F\d]+"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["xoxo", "xoxs"],
    },
    Rule {
        name: "slack-legacy-workspace-token",
        pattern: r#"xox[ar]-(?:\d-)?[0-9a-zA-Z]{8,48}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["xoxa", "xoxr"],
    },
    Rule {
        name: "slack-user-token",
        pattern: r#"xox[pe](?:-[0-9]{10,13}){3}-[a-zA-Z0-9-]{28,34}"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["xoxp-", "xoxe-"],
    },
    Rule {
        name: "slack-webhook-url",
        pattern: r#"(?:https?://)?hooks.slack.com/(?:services|workflows|triggers)/[A-Za-z0-9+/]{43,56}"#,
        confidence: Confidence::Shape,
        keywords: &["hooks.slack.com"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "snyk-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:snyk[_.-]?(?:(?:api|oauth)[_.-]?)?(?:key|token))(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["snyk"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sonar-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:sonar[_.-]?(login|token))(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}((?:squ_|sqp_|sqa_)?[a-z0-9=_\-]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["sonar"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sourcegraph-access-token",
        pattern: r#"(?i)\b(\b(sgp_(?:[a-fA-F0-9]{16}|local)_[a-fA-F0-9]{40}|sgp_[a-fA-F0-9]{40}|[a-fA-F0-9]{40})\b)(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["sgp_", "sourcegraph"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "square-access-token",
        pattern: r#"\b((?:EAAA|sq0atp-)[\w-]{22,60})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &["sq0atp-", "eaaa"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "squarespace-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:squarespace)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["squarespace"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "stripe-access-token",
        pattern: r#"\b((?:sk|rk)_(?:test|live|prod)_[a-zA-Z0-9]{10,99})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(2.0),
        keywords: &[
            "sk_test", "sk_live", "sk_prod", "rk_test", "rk_live", "rk_prod",
        ],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sumologic-access-id",
        pattern: r#"[\w.-]{0,50}?(?i:[\w.-]{0,50}?(?:(?-i:[Ss]umo|SUMO))(?:[ \t\w.-]{0,20})[\s'"]{0,3})(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(su[a-zA-Z0-9]{12})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["sumo"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "sumologic-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:(?-i:[Ss]umo|SUMO))(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{64})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["sumo"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "telegram-bot-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:telegr)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9]{5,16}:(?-i:A)[a-z0-9_\-]{34})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["telegr"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "travisci-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:travis)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{22})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["travis"],
    },
    Rule {
        name: "twilio-api-key",
        pattern: r#"SK[0-9a-fA-F]{32}"#,
        confidence: Confidence::Entropy(3.0),
        keywords: &["sk"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "twitch-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:twitch)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{30})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["twitch"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "twitter-access-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:twitter)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{45})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["twitter"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "twitter-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:twitter)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([0-9]{15,25}-[a-zA-Z0-9]{20,40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["twitter"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "twitter-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:twitter)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{25})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["twitter"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "twitter-api-secret",
        pattern: r#"(?i)[\w.-]{0,50}?(?:twitter)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{50})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["twitter"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "twitter-bearer-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:twitter)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(A{22}[a-zA-Z0-9%]{80,100})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["twitter"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "typeform-api-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:typeform)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(tfp_[a-z0-9\-_\.=]{59})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["tfp_"],
    },
    Rule {
        // Bound widened for Rust: {138,300} -> {138,}
        // Trailing delimiter widened for prose: see the generator.
        name: "vault-batch-token",
        pattern: r#"\b(hvb\.[\w-]{138,})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(4.0),
        keywords: &["hvb."],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "vault-service-token",
        pattern: r#"\b((?:hvs\.[\w-]{90,120}|s\.(?i:[a-z0-9]{24})))(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Entropy(3.5),
        keywords: &["hvs.", "s."],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "yandex-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:yandex)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(t1\.[A-Z0-9a-z_-]+[=]{0,2}\.[A-Z0-9a-z_-]{86}[=]{0,2})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["yandex"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "yandex-api-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:yandex)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(AQVN[A-Za-z0-9_\-]{35,38})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["yandex"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "yandex-aws-access-token",
        pattern: r#"(?i)[\w.-]{0,50}?(?:yandex)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}(YC[a-zA-Z0-9_\-]{38})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["yandex"],
    },
    Rule {
        // Trailing delimiter widened for prose: see the generator.
        name: "zendesk-secret-key",
        pattern: r#"(?i)[\w.-]{0,50}?(?:zendesk)(?:[ \t\w.-]{0,20})[\s'"]{0,3}(?:=|>|:{1,3}=|\|\||:|=>|\?=|,)[\x60'"\s=]{0,5}([a-z0-9]{40})(?:[\x60'"\s;,.!?)\]}]|\\[nr]|$)"#,
        confidence: Confidence::Shape,
        keywords: &["zendesk"],
    },
];
