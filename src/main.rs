use clap::Parser;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::task;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Path to the CSV file
    #[clap(short, long)]
    csv_file_path: String,

    /// Old host
    #[clap(short, long)]
    old_host: String,

    /// New host
    #[clap(short, long)]
    new_host: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command-line arguments
    let args = Args::parse();

    let csv_file_path = &args.csv_file_path;
    let old_host = &args.old_host;
    let new_host = &args.new_host;

    // Read and parse the CSV file
    let mut rdr = csv::Reader::from_path(csv_file_path)?;
    let records: Vec<csv::StringRecord> = rdr.records().collect::<Result<_, _>>()?;

    // Define the number of concurrent tasks
    let concurrency = num_cpus::get() / 4;
    let semaphore = Arc::new(Semaphore::new(concurrency));

    let mut handles = vec![];

    for record in records {
        let semaphore = Arc::clone(&semaphore);
        let old_email = record.get(0).unwrap().to_string();
        let old_password = record.get(1).unwrap().to_string();
        let new_email = record.get(2).unwrap().to_string();
        let new_password = record.get(3).unwrap().to_string();
        let old_host = old_host.clone();
        let new_host = new_host.clone();

        let handle = task::spawn(async move {
            let _permit = semaphore.acquire().await.unwrap();
            migrate_email(
                &old_host,
                &old_email,
                &old_password,
                &new_host,
                &new_email,
                &new_password,
            )
            .await;
        });

        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    println!("Migration complete.");

    Ok(())
}

async fn migrate_email(
    old_host: &str,
    old_email: &str,
    old_password: &str,
    new_host: &str,
    new_email: &str,
    new_password: &str,
) {
    let command = format!(
        "imapsync --host1 {} --user1 {} --password1 {} --host2 {} --user2 {} --password2 {} --ssl1 --noid",
        old_host, old_email, old_password, new_host, new_email, new_password
    );
    println!("Migrating {} to {}", old_email, new_email);

    let output = Command::new("sh")
        .arg("-c")
        .arg(&command)
        .output()
        .expect("failed to execute process");

    if output.status.success() {
        println!("Successfully migrated {}", old_email);
    } else {
        eprintln!(
            "Error migrating {}: {}",
            old_email,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}