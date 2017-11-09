use std::path::Path;
use std::env;
use ::fail;
use elfkit::*;
use std::fs::OpenOptions;
use ::goblin;
use std::io::{Read, Cursor};
use colored::*;


