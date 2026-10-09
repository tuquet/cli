use std::fs;
use crate::cli::{FakerGenerateArgs, FakerSubcommands};
use crate::ui::{badge_online, Card, Column, Table};
use specter_faker::{
    generate_id_only, generate_users, list_nationality_metadata, to_csv, to_json_value,
};

pub async fn handle(
    subcommand: Option<FakerSubcommands>,
    args: FakerGenerateArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    match subcommand {
        Some(FakerSubcommands::Generate(gen_args)) => {
            run_generate(gen_args).await?;
        }
        Some(FakerSubcommands::Card { gender, nat, avatar, domain, format }) => {
            run_card(gender.as_deref(), nat.as_deref(), avatar.as_deref(), domain.as_deref(), format.resolve()).await?;
        }
        Some(FakerSubcommands::Nationalities { table }) => {
            run_nationalities(table)?;
        }
        Some(FakerSubcommands::Id { nat, raw }) => {
            run_id(nat.as_deref(), raw)?;
        }
        Some(FakerSubcommands::Config { edit, show, domain, args }) => {
            manage_config(&args, edit, show, domain.as_deref())?;
        }
        None => {
            // Root invocation: specter faker [-n <count>] [--card] [--table] [--csv]
            run_generate(args).await?;
        }
    }

    Ok(())
}

pub async fn run_generate(args: FakerGenerateArgs) -> Result<(), Box<dyn std::error::Error>> {
    let gender_opt = args.gender.as_deref().filter(|&g| g != "all");
    let nat_opt = args.nat.as_deref().filter(|&n| n != "all");

    let data = generate_users(
        args.count,
        gender_opt,
        nat_opt,
        args.avatar.as_deref(),
        args.domain.as_deref(),
    );
    let users = data["results"].as_array().expect("results should be an array");

    let format_lower = args.format.to_lowercase();
    let effective_format = if args.card {
        "card"
    } else if args.table {
        "table"
    } else if args.csv {
        "csv"
    } else {
        format_lower.as_str()
    };

    match effective_format {
        "card" => {
            for u in users {
                render_single_card(u);
            }
        }
        "table" => {
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
        "csv" => {
            let csv_str = to_csv(users);
            if let Some(ref path) = args.output {
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
        _ => {
            // Default: pure JSON matching RandomUser standard {"results": [...], "info": {...}}
            let json_str = to_json_value(&data, true)?;
            if let Some(ref path) = args.output {
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
    }

    Ok(())
}

pub fn run_nationalities(table: bool) -> Result<(), Box<dyn std::error::Error>> {
    let list = list_nationality_metadata();
    if table {
        let columns = vec![
            Column { title: "Code".to_string(), min_width: 6, align_right: false },
            Column { title: "Country".to_string(), min_width: 18, align_right: false },
            Column { title: "ID Type".to_string(), min_width: 12, align_right: false },
            Column { title: "Phone Prefix".to_string(), min_width: 14, align_right: false },
            Column { title: "Timezone".to_string(), min_width: 10, align_right: false },
            Column { title: "Address Hierarchy".to_string(), min_width: 32, align_right: false },
        ];
        let mut t = Table::new(columns);
        for item in &list {
            t.add_row(vec![
                item.code.to_string(),
                item.country.to_string(),
                item.id_type.to_string(),
                item.phone_prefix.to_string(),
                item.timezone.to_string(),
                item.address_hierarchy.to_string(),
            ]);
        }
        let footer = format!("{} supported nationality provider(s)", list.len());
        println!("{}", t.with_footer(footer).render());
    } else {
        let json_str = to_json_value(&list, true)?;
        println!("{}", json_str);
    }
    Ok(())
}

pub fn run_id(nat: Option<&str>, raw: bool) -> Result<(), Box<dyn std::error::Error>> {
    let nat_opt = nat.filter(|&n| n != "all");
    let id_val = generate_id_only(nat_opt);

    if raw {
        if let Some(val_str) = id_val["value"].as_str() {
            println!("{}", val_str);
        } else {
            println!("{}", id_val["value"]);
        }
    } else {
        let json_str = to_json_value(&id_val, true)?;
        println!("{}", json_str);
    }
    Ok(())
}

pub async fn run_card(
    gender: Option<&str>,
    nat: Option<&str>,
    avatar: Option<&str>,
    domain: Option<&str>,
    format: crate::ui::OutputFormat,
) -> Result<(), Box<dyn std::error::Error>> {
    let gender_opt = gender.filter(|&g| g != "all");
    let nat_opt = nat.filter(|&n| n != "all");

    let data = generate_users(1, gender_opt, nat_opt, avatar, domain);
    let users = data["results"].as_array().expect("results is array");
    if let Some(user) = users.first() {
        if format.is_card() {
            render_single_card(user);
        } else {
            let json_str = to_json_value(user, true)?;
            println!("{}", json_str);
        }
    }

    Ok(())
}

pub fn manage_config(args: &[String], edit: bool, show: bool, set_domain: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(domain) = set_domain {
        return crate::config::ConfigController::set_key("faker", "default_domain", domain);
    }
    crate::config::ConfigController::handle_dispatch("faker", args, edit, show)
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
