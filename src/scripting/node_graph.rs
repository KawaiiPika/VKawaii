use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Showing a pin on a Node.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Pin {
    pub id: Uuid,
    pub name: String,
}

/// Throwing a Node into the Graph.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Node {
    pub id: Uuid,
    pub type_name: String,
    pub inputs: Vec<Pin>,
    pub outputs: Vec<Pin>,
    pub position: [f32; 2],
}

/// Linking up two Pins.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Connection {
    pub id: Uuid,
    pub from_pin: Uuid,
    pub to_pin: Uuid,
}

/// Laying out the whole Node graph.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct NodeGraph {
    pub nodes: Vec<Node>,
    pub connections: Vec<Connection>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_graph_serialization() {
        let node_a_input = Pin {
            id: Uuid::new_v4(),
            name: "Input".to_string(),
        };
        let node_a_output = Pin {
            id: Uuid::new_v4(),
            name: "Output".to_string(),
        };

        let node_b_input = Pin {
            id: Uuid::new_v4(),
            name: "Input".to_string(),
        };
        let node_b_output = Pin {
            id: Uuid::new_v4(),
            name: "Output".to_string(),
        };

        let node_a = Node {
            id: Uuid::new_v4(),
            type_name: "TestNode".to_string(),
            inputs: vec![node_a_input],
            outputs: vec![node_a_output],
            position: [0.0, 0.0],
        };

        let node_b = Node {
            id: Uuid::new_v4(),
            type_name: "TestNode".to_string(),
            inputs: vec![node_b_input],
            outputs: vec![node_b_output],
            position: [100.0, 100.0],
        };

        let connection = Connection {
            id: Uuid::new_v4(),
            from_pin: node_a.outputs[0].id,
            to_pin: node_b.inputs[0].id,
        };

        let graph = NodeGraph {
            nodes: vec![node_a, node_b],
            connections: vec![connection],
        };

        let serialized = serde_json::to_string(&graph).unwrap();
        let deserialized: NodeGraph = serde_json::from_str(&serialized).unwrap();

        assert_eq!(graph, deserialized);
    }
}
