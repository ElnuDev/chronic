use chrono::{NaiveDate, Utc};
use const_format::concatcp;
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{stdin, BufRead, BufReader, Read},
    path::Path,
    str::FromStr,
};
use uuid::Uuid;

const HABITCTL_DIR: &str = "/home/elnu/.habitctl";
const HABITCTL_HABITS: &str = concatcp!(HABITCTL_DIR, "/habits");
const HABITCTL_LOG: &str = concatcp!(HABITCTL_DIR, "/log");

const DIR: &str = "/home/elnu/.chronic";
const HABITS: &str = concatcp!(DIR, "/habits");
const LOG_DIR: &str = concatcp!(DIR, "/logs");

const NAME: &str = "chronic";

fn main() {
    let args: Vec<String> = env::args().collect();
    if !installed() {
        setup();
    } else if args.len() > 1 {
        if args.get(1).unwrap() == "log" && args.len() > 2 {
            let habits = get_habits();
            let entries = get_entries(args.get(2).unwrap());
            for entry in entries.iter() {
                println!("{}", entry.habit.description);
            }
        } else {
            // Just here to prevent the collapsible_if clippy warning
        }
    }
}

fn setup() {
    println!("Welcome to {}!", NAME);
    if habitctl_installed() {
        println!(
            "A habitctl installation has been detected in {}",
            HABITCTL_DIR
        );
        println!("Would you like to import it into {}? (Y/n)", NAME);
        let import = loop {
            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
            input = input.trim().to_lowercase();
            if input.is_empty() || input == "y" {
                break true;
            } else if input == "n" {
                break false;
            }
            println!("Invalid response.");
        };
        fs::create_dir_all(LOG_DIR).unwrap();
        if import {
            let habits = parse_habitctl_habits();
            let serialized_habits = serde_yaml::to_string(&habits).unwrap();
            fs::write(HABITS, serialized_habits).unwrap();

            let entries = parse_habitctl_log(&habits);
            let sorted_entries = sort_entries_by_date(&entries);

            for (date, entries) in sorted_entries.iter() {
                let serialized_entries = serde_yaml::to_string(&entries).unwrap();
                fs::write(format!("{}/{}", LOG_DIR, date), serialized_entries).unwrap();
            }
        }
        println!("Done!");
    }
}

fn sort_entries_by_date<'a>(entries: &'a [Entry<'a>]) -> HashMap<NaiveDate, Vec<&'a Entry<'a>>> {
    let mut sorted_entries = HashMap::new();

    for entry in entries.iter() {
        sorted_entries
            .entry(entry.date)
            .or_insert_with(Vec::new)
            .push(entry);
    }

    sorted_entries
}

fn installed() -> bool {
    Path::new(HABITS).is_file() && Path::new(LOG_DIR).is_dir()
}

fn habitctl_installed() -> bool {
    Path::new(HABITCTL_HABITS).is_file() && Path::new(HABITCTL_LOG).is_file()
}

fn get_habits() -> Vec<Habit> {
    let mut habits = File::open(HABITS).unwrap();
    let mut habits_contents = String::new();
    habits.read_to_string(&mut habits_contents).unwrap();
    serde_yaml::from_str(&habits_contents).unwrap()
}

fn get_entries(date: &str) -> Vec<Entry> {
    // TODO: Switch to Option<Vec<Entry>> to accommodate for invalid dates
    let mut log = File::open(format!("{}/{}", LOG_DIR, date)).unwrap();
    let mut log_contents = String::new();
    log.read_to_string(&mut log_contents).unwrap();
    serde_yaml::from_str(&log_contents).unwrap()
}

fn parse_habitctl_habits() -> Vec<Habit> {
    let habits = File::open(HABITCTL_HABITS).unwrap();
    let habits_reader = BufReader::new(habits);
    let mut habit_list = Vec::<Habit>::new();
    for line in habits_reader.lines().flatten() {
        let habit = Habit::from_habitctl_line(&line);
        if habit.is_none() {
            continue;
        }
        let habit = habit.unwrap();
        habit_list.push(habit);
    }
    habit_list
}

fn parse_habitctl_log(habits: &Vec<Habit>) -> Vec<Entry> {
    let log = File::open(HABITCTL_LOG).unwrap();
    let log_reader = BufReader::new(log);
    let mut entry_list = Vec::<Entry>::new();
    for line in log_reader.lines().flatten() {
        let entry = Entry::from_habitctl_line(&line, habits);
        if entry.is_none() {
            continue;
        }
        let entry = entry.unwrap();
        entry_list.push(entry);
    }
    entry_list
}

#[derive(Debug, Serialize, Deserialize)]
struct Habit<'a> {
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    uuid: Uuid,
    r#type: HabitType,
    description: String,
}

fn serialize_uuid<S>(uuid: &Uuid, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(&format_args!("{:?}", uuid))
}

fn deserialize_uuid<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: Deserializer<'de>,
{
    struct UuidVisitor;

    impl<'de> Visitor<'de> for UuidVisitor {
        type Value = Uuid;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string containing a UUID")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(Uuid::from_str(value).unwrap())
        }
    }

    deserializer.deserialize_any(UuidVisitor)
}

impl Habit {
    fn new(r#type: HabitType, description: String) -> Self {
        Self {
            r#type,
            description,
            uuid: Uuid::new_v4(),
        }
    }

    fn from_habitctl_line(line: &str) -> Option<Self> {
        let line = line.trim();
        // Immediately exit out if the line is empty after trimming,
        // since the next line.chars().next().unwrap() will panic if it is empty
        if line.is_empty() {
            return None;
        }
        let habit_type = HabitType::from_habitctl_char(line.chars().next().unwrap());
        // Return None if HabitType::from_char returns a None variant.
        // Otherwise, unwrap.
        let habit_type = habit_type?;
        // Remove the first character that habitctl uses for type distinction.
        let description = line[1..].trim().to_string();
        Some(Self::new(habit_type, description))
    }

    fn from_uuid(habits: &[Habit], uuid: Uuid) -> Option<&Self> {
        let mut i = 0;
        let mut matching_habit = None;
        while i < habits.len() {
            let habit = habits.get(i).unwrap();
            if habit.uuid == uuid {
                matching_habit = Some(habit);
                break;
            }
            i += 1;
        }
        matching_habit
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum HabitType {
    #[serde(rename = "just_track")]
    JustTrack,

    #[serde(rename = "daily")]
    Daily,

    #[serde(rename = "weekly")]
    Weekly,
}

impl HabitType {
    fn from_habitctl_char(char: char) -> Option<Self> {
        match char {
            '0' => Some(Self::JustTrack),
            '1' => Some(Self::Daily),
            '7' => Some(Self::Weekly),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Entry<'a> {
    #[serde(serialize_with = "serialize_naive_date")]
    #[serde(deserialize_with = "deserialize_naive_date")]
    date: NaiveDate,
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    habit: &'a Habit<'a>,
    entry_status: EntryStatus,
}

impl Entry<'_> {
    fn new(date: NaiveDate, habit: &'_ Habit<'_>, entry_status: EntryStatus) -> Self {
        Self {
            date,
            habit,
            entry_status,
        }
    }
    fn now(habit: &Habit, entry_status: EntryStatus) -> Self {
        Self::new(Utc::now().naive_utc().date(), habit, entry_status)
    }
    fn from_habitctl_line(line: &str, habits: &Vec<Habit>) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }
        const DATE_LENGTH: usize = 10;
        let date = NaiveDate::from_str(&line[..DATE_LENGTH]).unwrap();
        let habit = {
            let mut i = 0;
            let mut matching_habit = None;
            while i < habits.len() {
                let habit = habits.get(i).unwrap();
                if habit.description == line[DATE_LENGTH + 1..line.len() - 2] {
                    matching_habit = Some(habit.uuid);
                    break;
                }
                i += 1;
            }
            matching_habit
        }?;
        let entry_status = EntryStatus::from_habitctl_char(line.chars().last().unwrap())?;
        Some(Self::new(date, habit, entry_status))
    }
}

fn serialize_naive_date<S>(naive_date: &NaiveDate, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_str(&format_args!("{:?}", naive_date))
}

fn deserialize_naive_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    struct NaiveDateVisitor;

    impl<'de> Visitor<'de> for NaiveDateVisitor {
        type Value = NaiveDate;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a string containing a date")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(NaiveDate::from_str(value).unwrap())
        }
    }

    deserializer.deserialize_any(NaiveDateVisitor)
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
enum EntryStatus {
    #[serde(rename = "completed")]
    Completed,

    #[serde(rename = "not_completed")]
    NotCompleted,

    #[serde(rename = "skipped")]
    Skipped,
}

impl EntryStatus {
    fn from_habitctl_char(char: char) -> Option<Self> {
        match char {
            'y' => Some(Self::Completed),
            'n' => Some(Self::NotCompleted),
            's' => Some(Self::Skipped),
            _ => None,
        }
    }
}
