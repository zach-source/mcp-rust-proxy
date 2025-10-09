use dashmap::DashMap;

pub struct RequestRouter {
    // Maps resource/tool/prompt names to server names
    pub tool_to_server: DashMap<String, String>,
    pub resource_to_server: DashMap<String, String>,
    pub prompt_to_server: DashMap<String, String>,
}

impl RequestRouter {
    pub fn new() -> Self {
        Self {
            tool_to_server: DashMap::new(),
            resource_to_server: DashMap::new(),
            prompt_to_server: DashMap::new(),
        }
    }

    pub fn register_tool(&self, tool_name: String, server_name: String) {
        self.tool_to_server.insert(tool_name, server_name);
    }

    pub fn register_resource(&self, resource_uri: String, server_name: String) {
        self.resource_to_server.insert(resource_uri, server_name);
    }

    pub fn register_prompt(&self, prompt_name: String, server_name: String) {
        self.prompt_to_server.insert(prompt_name, server_name);
    }

    pub fn get_server_for_tool(&self, tool_name: &str) -> Option<String> {
        self.tool_to_server.get(tool_name).map(|v| v.clone())
    }

    pub fn get_server_for_resource(&self, resource_uri: &str) -> Option<String> {
        self.resource_to_server.get(resource_uri).map(|v| v.clone())
    }

    pub fn get_server_for_prompt(&self, prompt_name: &str) -> Option<String> {
        self.prompt_to_server.get(prompt_name).map(|v| v.clone())
    }

    pub fn unregister_server(&self, server_name: &str) {
        // Remove all entries for this server
        self.tool_to_server.retain(|_, v| v != server_name);
        self.resource_to_server.retain(|_, v| v != server_name);
        self.prompt_to_server.retain(|_, v| v != server_name);
    }

    pub fn clear(&self) {
        self.tool_to_server.clear();
        self.resource_to_server.clear();
        self.prompt_to_server.clear();
    }
}
