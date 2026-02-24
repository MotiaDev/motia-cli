use serde::Deserialize;

const REPO: &str = "MotiaDev/motia-iii-example";
const BRANCH: &str = "main";
const SKIP_FILES: &[&str] = &["package-lock.json", "README.md"];

#[derive(Debug, Deserialize)]
struct TreeResponse {
    tree: Vec<TreeEntry>,
}

#[derive(Debug, Deserialize)]
struct TreeEntry {
    path: String,
    #[serde(rename = "type")]
    entry_type: String,
}

pub fn raw_url(path: &str) -> String {
    format!("https://raw.githubusercontent.com/{}/{}/{}", REPO, BRANCH, path)
}

pub fn api_url() -> String {
    format!(
        "https://api.github.com/repos/{}/git/trees/{}?recursive=1",
        REPO, BRANCH
    )
}

pub async fn fetch_tree(client: &reqwest::Client, template_prefix: &str) -> anyhow::Result<Vec<String>> {
    let url = api_url();
    let res = client
        .get(&url)
        .header("User-Agent", "motia-cli")
        .send()
        .await?;

    if !res.status().is_success() {
        anyhow::bail!("Failed to fetch template repository: {}", res.status());
    }

    let data: TreeResponse = res.json().await?;
    let prefix = if template_prefix.is_empty() {
        String::new()
    } else {
        format!("{}/", template_prefix)
    };

    let files: Vec<String> = data
        .tree
        .into_iter()
        .filter(|e| {
            e.entry_type == "blob"
                && e.path.starts_with(&prefix)
                && !SKIP_FILES.iter().any(|s| e.path.ends_with(s))
        })
        .map(|e| e.path)
        .collect();

    Ok(files)
}

pub async fn download_file(client: &reqwest::Client, path: &str) -> anyhow::Result<String> {
    let url = raw_url(path);
    let res = client.get(&url).send().await?;

    if !res.status().is_success() {
        anyhow::bail!("Failed to download {}: {}", path, res.status());
    }

    let text = res.text().await?;
    Ok(text)
}
