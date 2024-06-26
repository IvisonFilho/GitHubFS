use serde::Deserialize;
use reqwest::blocking::Client;

#[derive(Debug, Deserialize)]
struct GitHubBranch {
    name: String,
}

fn main() {
    let owner = "OWNER"; // Substitua com o nome do usuário ou organização
    let repo = "GitHubFS"; // Substitua com o nome do repositório
    let token = "YOUR-TOKEN"; // Substitua com seu token de acesso do GitHub

    let api_url = format!(
        "https://api.github.com/repos/{}/{}/branches",
        owner, repo
    );

    let client = Client::builder()
        .user_agent("User") // Defina um User-Agent significativo aqui
        .build()
        .expect("Failed to create reqwest client");

    let response = send_github_request(&client, &api_url, token);

    match response {
        Ok(branches) => {
            for branch in branches {
                println!("{}", branch.name);
            }
        }
        Err(e) => {
            eprintln!("Failed to fetch branches: {}", e);
        }
    }
}

fn send_github_request(client: &Client, url: &str, token: &str) -> Result<Vec<GitHubBranch>, Box<dyn std::error::Error>> {
    let response = client
        .get(url)
        .header("Accept", "application/vnd.github.v3+json")
        .header("Authorization", format!("Bearer {}", token))
        .send()?;

    if response.status().is_success() {
        let branches: Vec<GitHubBranch> = response.json()?;
        Ok(branches)
    } else {
        Err(format!("GitHub API request failed with status: {}", response.status()).into())
    }
}
