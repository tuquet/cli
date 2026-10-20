use crate::cli::{FakerGenerateArgs, FakerSubcommands};
use crate::ui::{
    Card, ClackOption, Column, Table, badge_online, clack_cancel, clack_intro, clack_outro,
    clack_select, clack_step_ok, clack_text, is_interactive_tty,
};
use faker::{
    generate_id_only, generate_users, list_nationality_metadata, to_csv, to_json_value,
};
use std::fs;
use std::path::PathBuf;

pub async fn handle(
    subcommand: Option<FakerSubcommands>,
    args: FakerGenerateArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    match subcommand {
        Some(FakerSubcommands::Generate(gen_args)) => {
            run_generate(gen_args).await?;
        }
        Some(FakerSubcommands::Card {
            gender,
            nat,
            avatar,
            domain,
            format,
        }) => {
            run_card(
                gender.as_deref(),
                nat.as_deref(),
                avatar.as_deref(),
                domain.as_deref(),
                format.resolve(),
            )
            .await?;
        }
        Some(FakerSubcommands::Nationalities { table, json }) => {
            run_nationalities(table || (!json && is_interactive_tty()))?;
        }
        Some(FakerSubcommands::Id { nat, raw }) => {
            run_id(nat.as_deref(), raw)?;
        }
        Some(FakerSubcommands::Config {
            edit,
            show,
            domain,
            args,
        }) => {
            manage_config(&args, edit, show, domain.as_deref())?;
        }
        None => {
            let is_zero_arg_default = args.count == 1
                && args.gender.is_none()
                && args.nat.is_none()
                && args.domain.is_none()
                && !args.card
                && !args.table
                && !args.csv
                && !args.json
                && args.format.eq_ignore_ascii_case("auto")
                && args.output.is_none();

            if is_zero_arg_default && is_interactive_tty() {
                run_interactive_faker().await?;
            } else {
                run_generate(args).await?;
            }
        }
    }

    Ok(())
}

async fn run_interactive_faker() -> Result<(), Box<dyn std::error::Error>> {
    clack_intro(
        "tq faker",
        "Synthetic Persona & Identity Generator (CCCD • SSN • My Number)",
    );

    let actions = vec![
        ClackOption::new(
            "Generate 1 Synthetic Identity Card",
            "Full persona with validated National ID, address, phone & credentials",
            "Generate 1 Synthetic Identity Card",
        ),
        ClackOption::new(
            "Generate Batch Identity Dataset",
            "Generate multiple profiles in Clack Table, CSV, or JSON",
            "Generate Batch Identity Dataset",
        ),
        ClackOption::new(
            "Generate Validated National ID Only",
            "12-digit VN CCCD, 9-digit US SSN, or 12-digit JP My Number",
            "Generate Validated National ID Only",
        ),
        ClackOption::new(
            "Browse Supported Nationalities & Schemas",
            "Inspect VN, US, JP locale providers & address hierarchies",
            "Browse Supported Nationalities & Schemas",
        ),
        ClackOption::new(
            "Configure Default Faker Settings",
            "Inspect or update default_nat / default_domain in ~/.tq/faker/faker.json",
            "Configure Default Faker Settings",
        ),
    ];

    let Some(action_idx) = clack_select("Select a Faker operation", &actions, 0, 8)? else {
        clack_cancel("Operation cancelled.");
        return Ok(());
    };

    match action_idx {
        0 => {
            let Some(nat) = prompt_nationality_choice(true)? else {
                clack_cancel("Operation cancelled.");
                return Ok(());
            };
            let Some(gender) = prompt_gender_choice()? else {
                clack_cancel("Operation cancelled.");
                return Ok(());
            };
            let data = generate_users(1, gender.as_deref(), nat.as_deref(), Some("real"), None);
            if let Some(u) = data["results"].as_array().and_then(|a| a.first()) {
                render_single_card(u);
            }
            clack_outro("Synthetic identity card generated.");
        }
        1 => {
            let Some(nat) = prompt_nationality_choice(true)? else {
                clack_cancel("Operation cancelled.");
                return Ok(());
            };
            let Some(count_str) =
                clack_text("Number of profiles to generate", Some("5"), Some("5"))?
            else {
                clack_cancel("Operation cancelled.");
                return Ok(());
            };
            let count: u32 = count_str.trim().parse().unwrap_or(5).max(1);

            let fmt_opts = vec![
                ClackOption::new(
                    "Terminal Table",
                    "Render interactive Clack table in terminal",
                    "Terminal Table",
                ),
                ClackOption::new(
                    "Export CSV File",
                    "Write RFC-compliant CSV dataset to disk",
                    "Export CSV File",
                ),
                ClackOption::new(
                    "Export JSON File",
                    "Write RandomUser-compatible JSON dataset to disk",
                    "Export JSON File",
                ),
            ];
            let Some(fmt_idx) = clack_select("Select output destination", &fmt_opts, 0, 6)? else {
                clack_cancel("Operation cancelled.");
                return Ok(());
            };

            let (format_str, output_path) = match fmt_idx {
                1 => {
                    let Some(p) =
                        clack_text("CSV output file path", Some("users.csv"), Some("users.csv"))?
                    else {
                        clack_cancel("Operation cancelled.");
                        return Ok(());
                    };
                    ("csv".to_string(), Some(PathBuf::from(p)))
                }
                2 => {
                    let Some(p) = clack_text(
                        "JSON output file path",
                        Some("users.json"),
                        Some("users.json"),
                    )?
                    else {
                        clack_cancel("Operation cancelled.");
                        return Ok(());
                    };
                    ("json".to_string(), Some(PathBuf::from(p)))
                }
                _ => ("table".to_string(), None),
            };

            run_generate_inner(
                FakerGenerateArgs {
                    count,
                    gender: None,
                    nat,
                    avatar: Some("real".to_string()),
                    domain: None,
                    format: format_str,
                    card: false,
                    table: fmt_idx == 0,
                    csv: fmt_idx == 1,
                    json: fmt_idx == 2,
                    output: output_path,
                },
                true,
            )
            .await?;
        }
        2 => {
            let Some(nat) = prompt_nationality_choice(false)? else {
                clack_cancel("Operation cancelled.");
                return Ok(());
            };
            run_id_inner(nat.as_deref(), false, true)?;
        }
        3 => {
            run_nationalities_inner(true, true)?;
        }
        4 => {
            crate::config::ConfigController::interactive_service_config("faker")?;
        }
        _ => {}
    }

    Ok(())
}

fn prompt_nationality_choice(
    allow_all: bool,
) -> Result<Option<Option<String>>, Box<dyn std::error::Error>> {
    let mut opts = vec![
        ClackOption::new(
            "VN — Vietnam",
            "12-digit CCCD checksum, Vietnamese full name & 3-tier address",
            "VN (Vietnam)",
        ),
        ClackOption::new(
            "US — United States",
            "9-digit SSN checksum, US street address & state",
            "US (United States)",
        ),
        ClackOption::new(
            "JP — Japan",
            "12-digit My Number Modulo-11 checksum, Japanese prefecture & ward",
            "JP (Japan)",
        ),
    ];
    if allow_all {
        opts.push(ClackOption::new(
            "All — Mixed Nationalities",
            "Randomly sample across VN, US, and JP providers",
            "All (Mixed)",
        ));
    }
    match clack_select("Select target nationality", &opts, 0, 6)? {
        Some(0) => Ok(Some(Some("VN".to_string()))),
        Some(1) => Ok(Some(Some("US".to_string()))),
        Some(2) => Ok(Some(Some("JP".to_string()))),
        Some(_) => Ok(Some(None)),
        None => Ok(None),
    }
}

fn prompt_gender_choice() -> Result<Option<Option<String>>, Box<dyn std::error::Error>> {
    let opts = vec![
        ClackOption::new("All (Random)", "Any gender", "All (Random)"),
        ClackOption::new("Female", "Female persona", "Female"),
        ClackOption::new("Male", "Male persona", "Male"),
    ];
    match clack_select("Select gender filter", &opts, 0, 6)? {
        Some(1) => Ok(Some(Some("female".to_string()))),
        Some(2) => Ok(Some(Some("male".to_string()))),
        Some(_) => Ok(Some(None)),
        None => Ok(None),
    }
}

pub async fn run_generate(args: FakerGenerateArgs) -> Result<(), Box<dyn std::error::Error>> {
    run_generate_inner(args, false).await
}

async fn run_generate_inner(
    args: FakerGenerateArgs,
    intro_already_shown: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let gender_opt = args.gender.as_deref().filter(|&g| g != "all");
    let nat_opt = args.nat.as_deref().filter(|&n| n != "all");

    let data = generate_users(
        args.count,
        gender_opt,
        nat_opt,
        args.avatar.as_deref(),
        args.domain.as_deref(),
    );
    let users = data["results"]
        .as_array()
        .expect("results should be an array");

    let format_lower = args.format.to_lowercase();
    let effective_format = if args.json && args.output.is_none() {
        "json"
    } else if args.card {
        "card"
    } else if args.table {
        "table"
    } else if args.csv {
        "csv"
    } else if matches!(format_lower.as_str(), "card" | "table" | "csv" | "json") {
        format_lower.as_str()
    } else if is_interactive_tty() && args.output.is_none() {
        if args.count == 1 { "card" } else { "table" }
    } else {
        "json"
    };

    match effective_format {
        "card" => {
            if !intro_already_shown {
                clack_intro(
                    "tq faker",
                    &format!("Synthetic Persona Generator • {} profile(s)", users.len()),
                );
            }
            for u in users {
                render_single_card(u);
            }
            clack_outro(&format!(
                "Generated {} synthetic persona card(s).",
                users.len()
            ));
        }
        "table" => {
            if !intro_already_shown {
                clack_intro(
                    "tq faker",
                    &format!("Synthetic Persona Dataset • {} profile(s)", users.len()),
                );
            }
            let columns = vec![
                Column {
                    title: "STT".to_string(),
                    min_width: 4,
                    align_right: false,
                },
                Column {
                    title: "Họ và tên".to_string(),
                    min_width: 20,
                    align_right: false,
                },
                Column {
                    title: "Giới".to_string(),
                    min_width: 6,
                    align_right: false,
                },
                Column {
                    title: "Tuổi".to_string(),
                    min_width: 4,
                    align_right: true,
                },
                Column {
                    title: "CCCD / National ID".to_string(),
                    min_width: 14,
                    align_right: false,
                },
                Column {
                    title: "Email".to_string(),
                    min_width: 24,
                    align_right: false,
                },
                Column {
                    title: "Số điện thoại".to_string(),
                    min_width: 12,
                    align_right: false,
                },
                Column {
                    title: "Tỉnh / Thành phố".to_string(),
                    min_width: 14,
                    align_right: false,
                },
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
                "Generated {} profile(s) • Tip: use '-f csv -o users.csv' to export dataset",
                users.len()
            );
            table.with_footer(footer).print();
            clack_outro(&format!(
                "Rendered {} synthetic identity profile(s).",
                users.len()
            ));
        }
        "csv" => {
            let csv_str = to_csv(users);
            if let Some(ref path) = args.output {
                if !intro_already_shown {
                    clack_intro("tq faker", "CSV Dataset Exporter");
                }
                fs::write(path, csv_str.as_bytes())?;
                clack_step_ok(format!(
                    "Exported {} identity profile(s) to {}",
                    users.len(),
                    path.display()
                ));
                clack_outro("CSV export complete.");
            } else {
                println!("{}", csv_str);
            }
        }
        _ => {
            let json_str = to_json_value(&data, true)?;
            if let Some(ref path) = args.output {
                if !intro_already_shown {
                    clack_intro("tq faker", "JSON Dataset Exporter");
                }
                fs::write(path, json_str.as_bytes())?;
                clack_step_ok(format!(
                    "Exported {} identity profile(s) to {}",
                    users.len(),
                    path.display()
                ));
                clack_outro("JSON export complete.");
            } else {
                println!("{}", json_str);
            }
        }
    }

    Ok(())
}

pub fn run_nationalities(table: bool) -> Result<(), Box<dyn std::error::Error>> {
    run_nationalities_inner(table, false)
}

fn run_nationalities_inner(
    table: bool,
    intro_already_shown: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let list = list_nationality_metadata();
    if table {
        if !intro_already_shown {
            clack_intro("tq faker", "Supported Nationality Providers & ID Schemas");
        }
        let columns = vec![
            Column {
                title: "Code".to_string(),
                min_width: 6,
                align_right: false,
            },
            Column {
                title: "Country".to_string(),
                min_width: 18,
                align_right: false,
            },
            Column {
                title: "ID Type".to_string(),
                min_width: 12,
                align_right: false,
            },
            Column {
                title: "Phone Prefix".to_string(),
                min_width: 14,
                align_right: false,
            },
            Column {
                title: "Timezone".to_string(),
                min_width: 10,
                align_right: false,
            },
            Column {
                title: "Address Hierarchy".to_string(),
                min_width: 32,
                align_right: false,
            },
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
        t.with_footer(footer).print();
        clack_outro(&format!("{} nationality locale(s) ready.", list.len()));
    } else {
        let json_str = to_json_value(&list, true)?;
        println!("{}", json_str);
    }
    Ok(())
}

pub fn run_id(nat: Option<&str>, raw: bool) -> Result<(), Box<dyn std::error::Error>> {
    run_id_inner(nat, raw, false)
}

fn run_id_inner(
    nat: Option<&str>,
    raw: bool,
    intro_already_shown: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let nat_opt = nat.filter(|&n| n != "all");
    let id_val = generate_id_only(nat_opt);

    if raw {
        if let Some(val_str) = id_val["value"].as_str() {
            println!("{}", val_str);
        } else {
            println!("{}", id_val["value"]);
        }
    } else if is_interactive_tty() {
        if !intro_already_shown {
            clack_intro("tq faker id", "Validated National ID Checksum Generator");
        }
        let mut card = Card::new("SYNTHETIC NATIONAL ID");
        card.with_badge(badge_online("CHECKSUM VALID"));
        card.with_min_width(58);
        card.add_kv("Nationality", id_val["nat"].as_str().unwrap_or("VN"));
        card.add_kv("ID Schema", id_val["name"].as_str().unwrap_or("CCCD"));
        card.add_kv("Generated Value", id_val["value"].as_str().unwrap_or(""));
        card.print();
        clack_outro("Use '--raw' (-r) to output plain ID string for pipes.");
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
        if !format.is_json() {
            clack_intro("tq faker card", "Synthetic Identity Card Inspector");
            render_single_card(user);
            clack_outro("Synthetic persona verified.");
        } else {
            let json_str = to_json_value(user, true)?;
            println!("{}", json_str);
        }
    }

    Ok(())
}

pub fn manage_config(
    args: &[String],
    edit: bool,
    show: bool,
    set_domain: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
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
    let dob = if dob_raw.len() >= 10 {
        &dob_raw[..10]
    } else {
        dob_raw
    };
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
    card.add_kv(
        "Gender / Age",
        format!("{} • {} tuổi (DOB: {})", gender, age, dob),
    );
    card.add_kv("Job / Title", job);
    card.add_kv("Phone", phone);
    card.add_kv("Email", email);
    card.add_kv(
        "Credentials",
        format!("user: {} | pass: {}", username, password),
    );
    card.add_kv("Address", address_full);
    card.add_kv("Postal Code", postcode);

    card.with_footer(format!("UUID: {} • Avatar: {}", uuid, avatar_url));
    card.print();
}
