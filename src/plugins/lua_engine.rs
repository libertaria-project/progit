// SPDX-License-Identifier: Apache-2.0
// Copyright (c) 2025 Markus Maiwald

//! LuaJIT Plugin Engine
//!
//! [ARCH] LuaJIT runtime for executing Lua plugins.
//! Implements PluginEngine trait for plugin-sdk compatibility.

use super::sdk::{PluginEngine, PluginEvent, PluginMetadata};
use mlua::{Lua, Table, Value};
use std::collections::HashMap;

/// LuaJIT-based plugin engine
pub struct LuaPluginEngine {
    lua: Lua,
    plugins: HashMap<String, PluginMetadata>,
}

impl LuaPluginEngine {
    /// Create a new LuaJIT plugin engine
    pub fn new() -> Result<Self, String> {
        let lua = Lua::new();
        Ok(Self {
            lua,
            plugins: HashMap::new(),
        })
    }
    
    /// Convert PluginEvent to Lua table
    fn event_to_lua(&self, event: &PluginEvent) -> Result<Table, String> {
        let table = self.lua.create_table().map_err(|e| e.to_string())?;
        
        match event {
            PluginEvent::Startup => {
                table.set("type", "Startup").map_err(|e| e.to_string())?;
            }
            PluginEvent::IssueCreated { issue_id } => {
                table.set("type", "IssueCreated").map_err(|e| e.to_string())?;
                let data = self.lua.create_table().map_err(|e| e.to_string())?;
                data.set("issue_id", issue_id.clone()).map_err(|e| e.to_string())?;
                table.set("data", data).map_err(|e| e.to_string())?;
            }
            PluginEvent::IssueUpdated { issue_id } => {
                table.set("type", "IssueUpdated").map_err(|e| e.to_string())?;
                let data = self.lua.create_table().map_err(|e| e.to_string())?;
                data.set("issue_id", issue_id.clone()).map_err(|e| e.to_string())?;
                table.set("data", data).map_err(|e| e.to_string())?;
            }
            // Add other event types as needed
            _ => {
                table.set("type", "Custom").map_err(|e| e.to_string())?;
            }
        }
        
        Ok(table)
    }
}

impl PluginEngine for LuaPluginEngine {
    fn load_plugin(&mut self, name: &str, source: &str) -> Result<(), String> {
        // Execute the plugin source code
        let plugin: Table = self.lua
            .load(source)
            .eval()
            .map_err(|e| format!("Failed to load plugin {}: {}", name, e))?;
        
        // Extract metadata
        let metadata_table: Table = plugin
            .get("metadata")
            .map_err(|e| format!("Plugin {} missing metadata: {}", name, e))?;
        
        let metadata = PluginMetadata {
            name: metadata_table.get("name").unwrap_or_else(|_| name.to_string()),
            version: metadata_table.get("version").unwrap_or_else(|_| "0.0.0".to_string()),
            description: metadata_table.get("description").unwrap_or_else(|_| String::new()),
            author: metadata_table.get("author").unwrap_or_else(|_| String::new()),
            license: metadata_table.get("license").unwrap_or_else(|_| "Unknown".to_string()),
        };
        
        // Store plugin in Lua global registry
        self.lua.globals()
            .set(format!("__plugin_{}", name), plugin)
            .map_err(|e| format!("Failed to register plugin {}: {}", name, e))?;
        
        // Call on_load if it exists
        if let Ok(plugin_table) = self.lua.globals().get::<_, Table>(format!("__plugin_{}", name)) {
            if let Ok(on_load) = plugin_table.get::<_, mlua::Function>("on_load") {
                let _: Value = on_load.call(plugin_table.clone())
                    .map_err(|e| format!("Plugin {} on_load failed: {}", name, e))?;
            }
        }
        
        self.plugins.insert(name.to_string(), metadata);
        Ok(())
    }
    
    fn unload_plugin(&mut self, name: &str) -> Result<(), String> {
        // Call on_unload if it exists
        if let Ok(plugin_table) = self.lua.globals().get::<_, Table>(format!("__plugin_{}", name)) {
            if let Ok(on_unload) = plugin_table.get::<_, mlua::Function>("on_unload") {
                let _: Value = on_unload.call(plugin_table.clone())
                    .map_err(|e| format!("Plugin {} on_unload failed: {}", name, e))?;
            }
        }
        
        // Remove from registry
        self.lua.globals()
            .set(format!("__plugin_{}", name), Value::Nil)
            .map_err(|e| format!("Failed to unregister plugin {}: {}", name, e))?;
        
        self.plugins.remove(name);
        Ok(())
    }
    
    fn dispatch_event(&mut self, event: &PluginEvent) -> Result<Vec<serde_json::Value>, String> {
        let mut responses = Vec::new();
        let event_table = self.event_to_lua(event)?;
        
        // Send event to all loaded plugins
        for plugin_name in self.plugins.keys().cloned().collect::<Vec<_>>() {
            if let Ok(plugin_table) = self.lua.globals().get::<_, Table>(format!("__plugin_{}", plugin_name)) {
                if let Ok(on_event) = plugin_table.get::<_, mlua::Function>("on_event") {
                    // Call plugin's on_event handler
                    match on_event.call::<_, Value>((plugin_table.clone(), event_table.clone())) {
                        Ok(Value::Nil) | Ok(Value::Null) => {
                            // No response from plugin
                        }
                        Ok(value) => {
                            // Convert Lua value to JSON (simplified)
                            responses.push(serde_json::Value::String(
                                format!("{:?}", value)
                            ));
                        }
                        Err(e) => {
                            eprintln!("Plugin {} event handler error: {}", plugin_name, e);
                        }
                    }
                }
            }
        }
        
        Ok(responses)
    }
    
    fn execute_command(&mut self, plugin_name: &str, command: &str, args: &[String]) -> Result<String, String> {
        let plugin_table = self.lua.globals()
            .get::<_, Table>(format!("__plugin_{}", plugin_name))
            .map_err(|_| format!("Plugin {} not loaded", plugin_name))?;
        
        let execute_command = plugin_table
            .get::<_, mlua::Function>("execute_command")
            .map_err(|_| format!("Plugin {} has no execute_command function", plugin_name))?;
        
        // Convert args to Lua table
        let args_table = self.lua.create_table().map_err(|e| e.to_string())?;
        for (i, arg) in args.iter().enumerate() {
            args_table.set(i + 1, arg.clone()).map_err(|e| e.to_string())?;
        }
        
        // Call plugin command
        let result: String = execute_command
            .call((plugin_table, command, args_table))
            .map_err(|e| format!("Command execution failed: {}", e))?;
        
        Ok(result)
    }
    
    fn list_plugins(&self) -> Vec<PluginMetadata> {
        self.plugins.values().cloned().collect()
    }
}

impl Default for LuaPluginEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create LuaJIT engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_load_simple_plugin() {
        let mut engine = LuaPluginEngine::new().unwrap();
        
        let source = r#"
            local plugin = {
                metadata = {
                    name = "test",
                    version = "1.0.0",
                    description = "Test plugin"
                }
            }
            
            function plugin:on_load()
                return true
            end
            
            return plugin
        "#;
        
        assert!(engine.load_plugin("test", source).is_ok());
        assert_eq!(engine.list_plugins().len(), 1);
    }
    
    #[test]
    fn test_plugin_command() {
        let mut engine = LuaPluginEngine::new().unwrap();
        
        let source = r#"
            local plugin = {
                metadata = {
                    name = "echo",
                    version = "1.0.0"
                }
            }
            
            function plugin:execute_command(command, args)
                if command == "echo" then
                    return "Echo: " .. args[1]
                end
                return "Unknown command"
            end
            
            return plugin
        "#;
        
        engine.load_plugin("echo", source).unwrap();
        
        let result = engine.execute_command("echo", "echo", &["hello".to_string()]).unwrap();
        assert_eq!(result, "Echo: hello");
    }
}
