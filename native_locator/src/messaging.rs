// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::{
    logging::{LogLevel, LogMessage},
    utils::PythonEnv,
};
use env_logger::Builder;
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};

pub trait MessageDispatcher {
    fn was_environment_reported(&self, env: &PythonEnv) -> bool;
    fn report_environment_manager(&mut self, env: EnvManager) -> ();
    fn report_environment(&mut self, env: PythonEnvironment) -> ();
    fn exit(&mut self) -> ();
}

#[derive(Serialize, Deserialize, Copy, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(Debug)]
pub enum EnvManagerType {
    Conda,
    Pyenv,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug)]
pub struct EnvManager {
    pub executable_path: PathBuf,
    pub version: Option<String>,
    pub tool: EnvManagerType,
}

impl EnvManager {
    pub fn new(executable_path: PathBuf, version: Option<String>, tool: EnvManagerType) -> Self {
        Self {
            executable_path,
            version,
            tool,
        }
    }
}

impl Clone for EnvManager {
    fn clone(&self) -> Self {
        Self {
            executable_path: self.executable_path.clone(),
            version: self.version.clone(),
            tool: self.tool.clone(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug)]
pub struct EnvManagerMessage {
    pub jsonrpc: String,
    pub method: String,
    pub params: EnvManager,
}

impl EnvManagerMessage {
    pub fn new(params: EnvManager) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: "envManager".to_string(),
            params,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(Debug)]
pub enum PythonEnvironmentCategory {
    System,
    Homebrew,
    Conda,
    Pyenv,
    PyenvVirtualEnv,
    WindowsStore,
    WindowsRegistry,
    Pipenv,
    VirtualEnvWrapper,
    Venv,
    VirtualEnv,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[derive(Debug)]
pub struct PythonEnvironment {
    pub name: Option<String>,
    pub python_executable_path: Option<PathBuf>,
    pub category: PythonEnvironmentCategory,
    pub version: Option<String>,
    pub env_path: Option<PathBuf>,
    pub sys_prefix_path: Option<PathBuf>,
    pub env_manager: Option<EnvManager>,
    pub python_run_command: Option<Vec<String>>,
    /**
     * The project path for the Pipenv environment.
     */
    pub project_path: Option<PathBuf>,
}

impl PythonEnvironment {
    pub fn new(
        name: Option<String>,
        python_executable_path: Option<PathBuf>,
        category: PythonEnvironmentCategory,
        version: Option<String>,
        env_path: Option<PathBuf>,
        sys_prefix_path: Option<PathBuf>,
        env_manager: Option<EnvManager>,
        python_run_command: Option<Vec<String>>,
    ) -> Self {
        Self {
            name,
            python_executable_path,
            category,
            version,
            env_path,
            sys_prefix_path,
            env_manager,
            python_run_command,
            project_path: None,
        }
    }
    pub fn new_pipenv(
        python_executable_path: Option<PathBuf>,
        version: Option<String>,
        env_path: Option<PathBuf>,
        sys_prefix_path: Option<PathBuf>,
        env_manager: Option<EnvManager>,
        project_path: PathBuf,
    ) -> Self {
        Self {
            name: None,
            python_executable_path: python_executable_path.clone(),
            category: PythonEnvironmentCategory::Pipenv,
            version,
            env_path,
            sys_prefix_path,
            env_manager,
            python_run_command: match python_executable_path {
                Some(exe) => Some(vec![exe.to_string_lossy().to_string()]),
                None => None,
            },
            project_path: Some(project_path),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug)]
pub struct PythonEnvironmentMessage {
    pub jsonrpc: String,
    pub method: String,
    pub params: PythonEnvironment,
}

impl PythonEnvironmentMessage {
    pub fn new(params: PythonEnvironment) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: "pythonEnvironment".to_string(),
            params,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[derive(Debug)]
pub struct ExitMessage {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<()>,
}

impl ExitMessage {
    pub fn new() -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: "exit".to_string(),
            params: None,
        }
    }
}

pub struct JsonRpcDispatcher {
    pub reported_managers: HashSet<String>,
    pub reported_environments: HashSet<String>,
}
pub fn send_message<T: serde::Serialize>(message: T) -> () {
    let message = serde_json::to_string(&message).unwrap();
    print!(
        "Content-Length: {}\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{}",
        message.len(),
        message
    );
}

pub fn initialize_logger(log_level: LevelFilter) {
    Builder::new()
        .format(|_, record| {
            let level = match record.level() {
                log::Level::Debug => LogLevel::Debug,
                log::Level::Error => LogLevel::Error,
                log::Level::Info => LogLevel::Info,
                log::Level::Warn => LogLevel::Warning,
                _ => LogLevel::Debug,
            };
            send_message(LogMessage::new(
                format!("{}", record.args()).to_string(),
                level,
            ));
            Ok(())
        })
        .filter(None, log_level)
        .init();
}

impl JsonRpcDispatcher {}
impl MessageDispatcher for JsonRpcDispatcher {
    fn was_environment_reported(&self, env: &PythonEnv) -> bool {
        if let Some(key) = env.executable.as_os_str().to_str() {
            return self.reported_environments.contains(key);
        }
        false
    }

    fn report_environment_manager(&mut self, env: EnvManager) -> () {
        if let Some(key) = get_manager_key(&env) {
            if !self.reported_managers.contains(&key) {
                self.reported_managers.insert(key);
                send_message(EnvManagerMessage::new(env));
            }
        }
    }
    fn report_environment(&mut self, env: PythonEnvironment) -> () {
        if let Some(key) = get_environment_key(&env) {
            if !self.reported_environments.contains(&key) {
                self.reported_environments.insert(key);
                send_message(PythonEnvironmentMessage::new(env.clone()));
            }
            if let Some(manager) = env.env_manager {
                self.report_environment_manager(manager);
            }
        }
    }
    fn exit(&mut self) -> () {
        send_message(ExitMessage::new());
    }
}

pub fn create_dispatcher() -> JsonRpcDispatcher {
    JsonRpcDispatcher {
        reported_managers: HashSet::new(),
        reported_environments: HashSet::new(),
    }
}

fn get_environment_key(env: &PythonEnvironment) -> Option<String> {
    match env.python_executable_path.clone() {
        Some(key) => Some(key.as_os_str().to_str()?.to_string()),
        None => match env.env_path.clone() {
            Some(key) => Some(key.as_os_str().to_str().unwrap().to_string()),
            None => None,
        },
    }
}

fn get_manager_key(manager: &EnvManager) -> Option<String> {
    Some(manager.executable_path.to_str()?.to_string())
}