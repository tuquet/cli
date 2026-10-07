use std::fs;
use std::path::PathBuf;
use crate::cli::FakerSubcommands;
use crate::ui::{badge_online, Card, Column, Table};
use tuquet_faker::{generate_users, to_csv, to_json, FakerConfig};

pub async fn handle(subcommand: Option<FakerSubcommands>) -> Result<(), Box<dyn std::error::Error>> {
    match subcommand {
        Some(FakerSubcommands::Generate {
            count,
            gender,
            nat,
            avatar,
            domain,
            format,
            output,
        }) => {
            run_generate(count, gender.as_deref(), nat.as_deref(), avatar.as_deref(), domain.as_deref(), &format, output).await?;
        }
        Some(FakerSubcommands::Card { gender, nat, avatar, domain }) => {
            run_card(gender.as_deref(), nat.as_deref(), avatar.as_deref(), domain.as_deref()).await?;
        }
        Some(FakerSubcommands::Config { edit, show, domain }) => {
            manage_config(edit, show, domain.as_deref())?;
        }
        None => {
            // Default to generating 1 profile in Card view
            run_card(None, Some("VN"), Some("real"), None).await?;
        }
    }

    Ok(())
}

pub async fn run_generate(
    count: u32,
    gender: Option<&str>,
    nat: Option<&str>,
    avatar: Option<&str>,
    domain: Option<&str>,
    format: &str,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let gender_opt = gender.filter(|&g| g != "all");
    let nat_opt = nat.filter(|&n| n != "all");

    let data = generate_users(count, gender_opt, nat_opt, avatar, domain);
    let users = data["results"].as_array().expect("results should be an array");

    match format.to_lowercase().as_str() {
        "json" => {
            let json_str = to_json(users, true)?;
            if let Some(ref path) = output {
                fs::write(path, json_str.as_bytes())?;
                println!(
                    "{} Exported {} identity profile(s) to {:?}",
                    badge_online("SUCCESS"),
                    users.len(),
                    path
                );
            } else {
                println!("{}", json_str);
            }
        }
        "csv" => {
            let csv_str = to_csv(users);
            if let Some(ref path) = output {
                fs::write(path, csv_str.as_bytes())?;
                println!(
                    "{} Exported {} identity profile(s) to {:?}",
                    badge_online("SUCCESS"),
                    users.len(),
                    path
                );
            } else {
                println!("{}", csv_str);
            }
        }
        "card" => {
            for u in users {
                render_single_card(u);
            }
        }
        _ => {
            // Default: Modern Table
            let columns = vec![
                Column { title: "STT".to_string(), min_width: 4, align_right: false },
                Column { title: "Họ và tên".to_string(), min_width: 20, align_right: false },
                Column { title: "Giới".to_string(), min_width: 6, align_right: false },
                Column { title: "Tuổi".to_string(), min_width: 4, align_right: true },
                Column { title: "CCCD / National ID".to_string(), min_width: 14, align_right: false },
                Column { title: "Email".to_string(), min_width: 24, align_right: false },
                Column { title: "Số điện thoại".to_string(), min_width: 12, align_right: false },
                Column { title: "Tỉnh / Thành phố".to_string(), min_width: 14, align_right: false },
            ];

            let mut table = Table::new(columns);

            for (i, u) in users.iter().enumerate() {
                let first = u["name"]["first"].as_str().unwrap_or("");
                let last = u["name"]["last"].as_str().unwrap_or("");
                let full_name = format!("{} {}", first, last);
                let gender_str = u["gender"].as_str().unwrap_or("");
                let age = u["dob"]["age"].to_string();
                let cccd = u["id"]["value"].as_str().unwrap_or("");
                let email = u["email"].as_str().unwrap_or("");
                let phone = u["phone"].as_str().unwrap_or("");
                let city = u["location"]["city"].as_str().unwrap_or("");

                table.add_row(vec![
                    (i + 1).to_string(),
                    full_name,
                    gender_str.to_string(),
                    age,
                    cccd.to_string(),
                    email.to_string(),
                    phone.to_string(),
                    city.to_string(),
                ]);
            }

            let footer = format!(
                "Generated {} profile(s) in 0ms • Tip: use '-f csv -o users.csv' to export dataset",
                users.len()
            );
            println!("{}", table.with_footer(footer).render());
        }
    }

    Ok(())
}

pub async fn run_card(
    gender: Option<&str>,
    nat: Option<&str>,
    avatar: Option<&str>,
    domain: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let gender_opt = gender.filter(|&g| g != "all");
    let nat_opt = nat.filter(|&n| n != "all");

    let data = generate_users(1, gender_opt, nat_opt, avatar, domain);
    let users = data["results"].as_array().expect("results is array");
    if let Some(user) = users.first() {
        render_single_card(user);
    }

    Ok(())
}

pub fn manage_config(edit: bool, show: bool, set_domain: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = FakerConfig::load();
    let config_path = FakerConfig::config_path();

    if let Some(domain) = set_domain {
        let clean = tuquet_faker::config::clean_domain(domain);
        if !clean.is_empty() {
            config.default_domain = clean.clone();
            if !config.email_domains.contains(&clean) {
                config.email_domains.insert(0, clean);
            }
            config.save()?;
            println!("{} Default email domain set to: \x1b[1;36m@{}\x1b[0m", badge_online("SAVED"), config.default_domain);
        }
    }

    if edit {
        crate::config::ConfigRegistry::open_in_editor(&config_path)?;
    } else if show || set_domain.is_some() {
        println!();
        let mut card = Card::new("FAKER CONFIGURATION");
        card.with_badge(badge_online("SSOT READY"));
        card.with_min_width(70);
        card.add_kv("Config File", config_path.to_string_lossy().to_string());
        card.add_kv("Default Domain", format!("@{}", config.default_domain));
        card.add_kv("Domain Pool", config.email_domains.iter().map(|d| format!("@{}", d)).collect::<Vec<_>>().join(", "));
        card.add_kv("Email Pattern", &config.email_pattern);
        card.add_kv("Default Locale", &config.default_nat);
        card.add_kv("Default Avatar", &config.default_avatar);
        card.with_footer("Tip: edit with 'specter faker config --edit' or quick set via '-d <domain>'");
        card.print();
        println!();
    } else {
        println!("{}", config_path.display());
    }

    Ok(())
}

fn render_single_card(u: &serde_json::Value) {
    let first = u["name"]["first"].as_str().unwrap_or("");
    let last = u["name"]["last"].as_str().unwrap_or("");
    let full_name = format!("{} {}", first, last);
    let gender = u["gender"].as_str().unwrap_or("");
    let dob_raw = u["dob"]["date"].as_str().unwrap_or("");
    let dob = if dob_raw.len() >= 10 { &dob_raw[..10] } else { dob_raw };
    let age = u["dob"]["age"].to_string();
    let cccd = u["id"]["value"].as_str().unwrap_or("");
    let email = u["email"].as_str().unwrap_or("");
    let username = u["login"]["username"].as_str().unwrap_or("");
    let password = u["login"]["password"].as_str().unwrap_or("");
    let phone = u["phone"].as_str().unwrap_or("");
    let street_num = u["location"]["street"]["number"].to_string();
    let street_name = u["location"]["street"]["name"].as_str().unwrap_or("");
    let ward = u["location"]["ward"].as_str().unwrap_or("");
    let district = u["location"]["district"].as_str().unwrap_or("");
    let city = u["location"]["city"].as_str().unwrap_or("");
    let postcode = u["location"]["postcode"].as_str().unwrap_or("");
    let job = u["job"].as_str().unwrap_or("");
    let avatar_url = u["picture"]["large"].as_str().unwrap_or("");
    let uuid = u["login"]["uuid"].as_str().unwrap_or("");

    let id_name = u["id"]["name"].as_str().unwrap_or("ID");
    let badge_text = match id_name {
        "CCCD" => "SYNTHETIC CCCD VERIFIED",
        "SSN" => "SYNTHETIC SSN VERIFIED",
        "My Number" => "SYNTHETIC MY NUMBER VERIFIED",
        _ => "SYNTHETIC IDENTITY VERIFIED",
    };

    let street_full = format!("{} {}", street_num, street_name).trim().to_string();
    let parts: Vec<&str> = [street_full.as_str(), ward, district, city]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();
    let address_full = parts.join(", ");

    let mut card = Card::new(&full_name);
    card.with_badge(badge_online(badge_text));
    card.with_min_width(70);

    card.add_kv(format!("{} / ID", id_name), cccd);
    card.add_kv("Gender / Age", format!("{} • {} tuổi (DOB: {})", gender, age, dob));
    card.add_kv("Job / Title", job);
    card.add_kv("Phone", phone);
    card.add_kv("Email", email);
    card.add_kv("Credentials", format!("user: {} | pass: {}", username, password));
    card.add_kv("Address", address_full);
    card.add_kv("Postal Code", postcode);

    card.with_footer(format!("UUID: {} • Avatar: {}", uuid, avatar_url));
    card.print();
}
