use chrono::{DateTime, NaiveDate, Utc};
use const_format::concatcp;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;
use std::{
    fs::File,
    io::{stdin, BufRead, BufReader},
    path::Path,
};
use uuid::Uuid;

const HABITCTL_DIR: &str = "/home/elnu/.habitctl";
const HABITCTL_HABITS: &str = concatcp!(HABITCTL_DIR, "/habits");
const HABITCTL_LOG: &str = concatcp!(HABITCTL_DIR, "/log");

const DIR: &str = "/home/elnu/.chronic";
const HABITS: &str = concatcp!(DIR, "/habits");
const LOG: &str = concatcp!(DIR, "/log");

const NAME: &str = "chronic";

fn main() {
    if !installed() {
        setup();
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
        if import {
            let habits = parse_habitctl_habits();
            println!("{}", serde_yaml::to_string(&habits).unwrap());
            let entries = parse_habitctl_log(&habits);
            println!("{}", serde_yaml::to_string(&entries).unwrap())
        }
    }
}

fn installed() -> bool {
    Path::new(HABITS).is_file() && Path::new(LOG).is_file()
}

fn habitctl_installed() -> bool {
    Path::new(HABITCTL_HABITS).is_file() && Path::new(HABITCTL_LOG).is_file()
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
struct Habit {
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
struct Entry {
    #[serde(serialize_with = "serialize_naive_date")]
    #[serde(deserialize_with = "deserialize_naive_date")]
    date: NaiveDate,
    #[serde(serialize_with = "serialize_uuid")]
    #[serde(deserialize_with = "deserialize_uuid")]
    habit: Uuid,
    entry_status: EntryStatus,
}

impl Entry {
    fn new(date: NaiveDate, habit: Uuid, entry_status: EntryStatus) -> Self {
        Self {
            date,
            habit,
            entry_status,
        }
    }
    fn now(habit: Uuid, entry_status: EntryStatus) -> Self {
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
