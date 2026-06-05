use crate::scripting::node_graph::{Node, NodeGraph};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

/// The Actionexecutor Runs Node Graphs Without Blocking things.
pub struct ActionExecutor {
    pub graph: NodeGraph,
    execution_queue: VecDeque<Uuid>,
    pin_to_node_map: HashMap<Uuid, Uuid>,
}

impl ActionExecutor {
    pub fn new(graph: NodeGraph) -> Self {
        let mut pin_to_node_map = HashMap::new();
        for node in &graph.nodes {
            for pin in &node.inputs {
                pin_to_node_map.insert(pin.id, node.id);
            }
        }

        Self {
            graph,
            execution_queue: VecDeque::new(),
            pin_to_node_map,
        }
    }

    /// Kicking off Execution Starting from nodes of a Specific type.
    pub fn trigger(&mut self, trigger_type: &str) {
        for node in &self.graph.nodes {
            if node.type_name == trigger_type {
                self.execution_queue.push_back(node.id);
            }
        }
    }

    /// Ticking the execution Engine, Chewing Through one Node per Call to stay Non-blocking.
    pub fn update(&mut self) {
        if let Some(node_id) = self.execution_queue.pop_front() {
            let node = self.graph.nodes.iter().find(|n| n.id == node_id).cloned();
            if let Some(node) = node {
                self.execute_node(&node);
            }
        }
    }

    fn execute_node(&mut self, node: &Node) {
        match node.type_name.as_str() {
            "LogMessage" => {
                println!("ActionExecutor: LogMessage node executed.");
            }
            "PlayAnimation" => {
                println!("ActionExecutor: PlayAnimation node executed.");
            }
            "SetBlendshape" => {
                println!("ActionExecutor: SetBlendshape node executed.");
            }
            "HotkeyTrigger" => {
                // Triggers themselves don't do Much When "executed"
                // Just Passing the flow along.
            }
            _ => {}
        }

        // Keeping the Graph Execution rolling by Queuing up
        // Any Nodes Hooked Into the outputs of the Current node.
        for output in &node.outputs {
            for conn in &self.graph.connections {
                if conn.from_pin == output.id {
                    if let Some(target_node) = self.find_node_by_input_pin(conn.to_pin) {
                        self.execution_queue.push_back(target_node.id);
                    }
                }
            }
        }
    }

    fn find_node_by_input_pin(&self, pin_id: Uuid) -> Option<&Node> {
        self.pin_to_node_map
            .get(&pin_id)
            .and_then(|node_id| self.graph.nodes.iter().find(|n| n.id == *node_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scripting::node_graph::{Connection, Pin};

    #[test]
    fn test_action_executor_execution_flow() {
        let trigger_out = Pin {
            id: Uuid::new_v4(),
            name: "Out".to_string(),
        };
        let action_in = Pin {
            id: Uuid::new_v4(),
            name: "In".to_string(),
        };
        let action_out = Pin {
            id: Uuid::new_v4(),
            name: "Out".to_string(),
        };
        let second_action_in = Pin {
            id: Uuid::new_v4(),
            name: "In".to_string(),
        };

        let trigger_node = Node {
            id: Uuid::new_v4(),
            type_name: "HotkeyTrigger".to_string(),
            inputs: vec![],
            outputs: vec![trigger_out.clone()],
            position: [0.0, 0.0],
        };

        let action_node = Node {
            id: Uuid::new_v4(),
            type_name: "LogMessage".to_string(),
            inputs: vec![action_in.clone()],
            outputs: vec![action_out.clone()],
            position: [150.0, 0.0],
        };

        let second_action_node = Node {
            id: Uuid::new_v4(),
            type_name: "PlayAnimation".to_string(),
            inputs: vec![second_action_in.clone()],
            outputs: vec![],
            position: [300.0, 0.0],
        };

        let conn1 = Connection {
            id: Uuid::new_v4(),
            from_pin: trigger_out.id,
            to_pin: action_in.id,
        };

        let conn2 = Connection {
            id: Uuid::new_v4(),
            from_pin: action_out.id,
            to_pin: second_action_in.id,
        };

        let graph = NodeGraph {
            nodes: vec![
                trigger_node.clone(),
                action_node.clone(),
                second_action_node.clone(),
            ],
            connections: vec![conn1, conn2],
        };

        let mut executor = ActionExecutor::new(graph);

        // Kicking off Execution
        executor.trigger("HotkeyTrigger");
        assert_eq!(executor.execution_queue.len(), 1);
        assert_eq!(executor.execution_queue[0], trigger_node.id);

        // Update 1: Running Trigger, queuing Action 1
        executor.update();
        assert_eq!(executor.execution_queue.len(), 1);
        assert_eq!(executor.execution_queue[0], action_node.id);

        // Update 2: Running Action 1, Queuing Action 2
        executor.update();
        assert_eq!(executor.execution_queue.len(), 1);
        assert_eq!(executor.execution_queue[0], second_action_node.id);

        // Update 3: Running Action 2, queue Empty
        executor.update();
        assert!(executor.execution_queue.is_empty());
    }
}
