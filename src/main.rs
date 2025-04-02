use std::path::Path;
use std::fs;
use std::io;
use encoding_rs::WINDOWS_1252;

#[derive(Debug)]
struct Account {
	number: u32,
	name: String,
	in_balance: f64,
	out_balance: f64,
}

#[derive(Debug)]
struct Verification {
	serie: String,
	number: u32,
	date: String,
	text: String,
	transactions: Vec<Transaction>,
}

#[derive(Debug)]
struct Transaction {
	account: u32,
	amount: f64,
}

fn read_sie_file<P: AsRef<Path>>(path: P) -> io::Result<String> {
	let raw_bytes = fs::read(path)?;
	let (cow, _, _) = WINDOWS_1252.decode(&raw_bytes);
	Ok(cow.into_owned())
}

fn clean_string(s: String) -> String {
	s.trim_matches('"')
		.trim()
		.replace('„', "ä") // fallback för felteckenkodningar
		.replace('”', "ö") // fallback för felteckenkodningar
		.replace('™', "Ö") // fallback för felteckenkodningar
		.replace('†', "å") // fallback för felteckenkodningar
		.replace('\u{8f}', "Å") // fallback för felteckenkodningar
		.replace('�', "?") // osäkra tecken
}

fn parse_line(s: &str) -> Vec<String> {
	let mut ret : Vec<String> = Vec::new();

	let mut inside_quote : bool = false;
	let mut current_string : String = String::new();
	for ch in s.chars() {
		match ch {
			'"' => {
				if inside_quote {
					inside_quote = false;
				} else {
					inside_quote = true;
				}
			},
			' ' => {
				if inside_quote {
					// just read.
					current_string.push(ch);
				} else {
					ret.push(current_string);
					current_string = String::new();
				}
			},
			_ => {
				current_string.push(ch);
			}
		}
	}

	ret.push(current_string);

	return ret;
}

fn find_account(accounts : &Vec<Account>, number : u32) -> Option<usize> {
	for (idx, account) in accounts.iter().enumerate() {
		if account.number == number {
			return Some(idx);
		}
	}
	return None;
}

fn calculate_out_balances(accounts : &mut Vec<Account>, verifications : &Vec<Verification>) {
	for account in accounts.iter_mut() {
		account.out_balance = account.in_balance;
	}
	for verification in verifications {
		for transaction in &verification.transactions {
			match find_account(&accounts, transaction.account) {
				Some(idx) => {
					accounts[idx].out_balance += transaction.amount;
				},
				None => {
					println!("Could not find account for transaction {}, {:?}", transaction.account, transaction);
				},
			}
		}
	}
}

fn parse_sie_file<P: AsRef<Path>>(path: P) -> io::Result<(Vec<Verification>, Vec<Account>)> {
	let contents = read_sie_file(path)?;

	let mut verifications = Vec::new();
	let mut accounts = Vec::new();
	let mut current_ver: Option<Verification> = None;

	for line in contents.lines() {

		if line.starts_with("#VER") {
			if let Some(ver) = current_ver.take() {
				verifications.push(ver);
			}

			let parts: Vec<String> = parse_line(line);
			println!("{:?}", parts);
			current_ver = Some(Verification {
				serie: clean_string(parts[1].clone()),
				number: parts[2].parse().unwrap_or(0),
				date: clean_string(parts[3].clone()),
				text: clean_string(parts[4].clone()),
				transactions: Vec::new(),
			});
		} else if line.starts_with("#IB") {

			let parts: Vec<String> = parse_line(line);

			let year : u32 = parts[1].parse().unwrap_or(0);
			let account_no : u32 = parts[2].parse().unwrap_or(0);
			let amount : f64 = parts[3].parse().unwrap_or(0.0);

			if year == 0 {
				match find_account(&accounts, account_no) {
					Some(idx) => {
						accounts[idx].in_balance = amount;
					},
					None => {
						println!("Cannot find account {}", account_no);
					}
				}
			}

		} else if line.starts_with("#KONTO") {
			let parts: Vec<String> = parse_line(line);
			let name : String = clean_string(parts[2].clone());
			let number : u32 = parts[1].parse().unwrap_or(0);
			let in_balance : f64 = 0.0;
			let out_balance : f64 = 0.0;
			accounts.push(Account {number, name, in_balance, out_balance});
		} else if line.starts_with("#TRANS") {
			if let Some(ref mut ver) = current_ver {
				let parts: Vec<String> = parse_line(line);
				let account = parts[1].parse().unwrap_or(0);
				let amount: f64 = parts.last().unwrap().parse().unwrap_or(0.0);
				ver.transactions.push(Transaction { account, amount });
			}
		}
	}

	if let Some(ver) = current_ver {
		verifications.push(ver);
	}

	calculate_out_balances(&mut accounts, &verifications);

	Ok((verifications, accounts))
}

fn main() {
	let path = "cc.se";
	match parse_sie_file(path) {
		Ok((verifications, accounts)) => {
			for ver in verifications {
				println!("{:?}", ver);
			}
			for account in accounts {
				if account.in_balance != 0.0 || account.out_balance != 0.0 {
					println!("{:?}", account);
				}
			}
		}
		Err(e) => eprintln!("Fel vid inläsning: {}", e),
	}
}

