// use serde::{Serialize, Deserialize};
// use std::error::Error;
// use std::fs;

// use crate::utils::task::Task;

// #[derive(Serialize, Deserialize)]
// pub struct Echo<'a> {
//     pub name: &'a str,
// }

// // id
// // parse
// // execute
// // out_of_spec
// // apply

// impl Task for Echo<'_> {
//     fn id(&self) -> &str {
//         &self.name.clone()
//     }

//     fn parse(&self) -> Result<(), Box<dyn Error>>{
//         let file_path = format!("/duties/{}.yaml", &self.id());
//         let contents = fs::read_to_string(file_path)?;
//         let _parsed_duty: EchoDuty = serde_yaml::from_str(&contents)?;
//         Ok(())
//     }

//     fn execute(&self) -> Result<(),Box<dyn Error>>{
//         match self.id() {
//             "echo" => {
//                 Ok(())
//             },
//             _ => {
//                 Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid module")))
//             }
//         }
//     }

//     fn out_of_spec(&self) -> Result<(), Box<dyn Error>>{
//         Ok(())
//     }

//     fn apply(&self) -> Result<(), Box<dyn Error>>{
//         Ok(())
//     }

// }

