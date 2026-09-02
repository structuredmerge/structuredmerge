use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct NodeSignature(pub Vec<String>);

impl NodeSignature {
    pub fn owner_path(path: impl Into<String>) -> Self {
        Self(vec!["owner_path".to_string(), path.into()])
    }

    pub fn content_identity(content: impl AsRef<str>) -> Option<Self> {
        let normalized = content.as_ref().trim();
        if normalized.is_empty() {
            return None;
        }
        Some(Self(vec!["content_identity".to_string(), normalized.to_string()]))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIdentity {
    pub primary: Option<NodeSignature>,
    pub aliases: Vec<NodeSignature>,
}

impl NodeIdentity {
    pub fn new(primary: Option<NodeSignature>) -> Self {
        Self { primary, aliases: Vec::new() }
    }

    pub fn with_alias(mut self, alias: Option<NodeSignature>) -> Self {
        if let Some(alias) = alias
            && self.primary.as_ref() != Some(&alias)
            && !self.aliases.contains(&alias)
        {
            self.aliases.push(alias);
        }
        self
    }

    fn signatures(&self) -> impl Iterator<Item = &NodeSignature> {
        self.primary.iter().chain(&self.aliases)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIdentityMatch {
    pub template_index: usize,
    pub destination_index: usize,
    pub signature: NodeSignature,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeIdentityMatchResult {
    pub matched: Vec<NodeIdentityMatch>,
    pub unmatched_template: Vec<usize>,
    pub unmatched_destination: Vec<usize>,
}

pub fn match_node_identities(
    template: &[NodeIdentity],
    destination: &[NodeIdentity],
) -> NodeIdentityMatchResult {
    let mut destination_by_signature: HashMap<NodeSignature, VecDeque<usize>> = HashMap::new();
    for (index, identity) in destination.iter().enumerate() {
        for signature in identity.signatures() {
            destination_by_signature.entry(signature.clone()).or_default().push_back(index);
        }
    }

    let mut matched_destination = HashSet::new();
    let mut matched = Vec::new();
    let mut unmatched_template = Vec::new();
    for (template_index, identity) in template.iter().enumerate() {
        let selected = identity.signatures().find_map(|signature| {
            let candidates = destination_by_signature.get_mut(signature)?;
            while let Some(destination_index) = candidates.pop_front() {
                if matched_destination.insert(destination_index) {
                    return Some((destination_index, signature.clone()));
                }
            }
            None
        });
        if let Some((destination_index, signature)) = selected {
            matched.push(NodeIdentityMatch { template_index, destination_index, signature });
        } else {
            unmatched_template.push(template_index);
        }
    }

    let unmatched_destination =
        (0..destination.len()).filter(|index| !matched_destination.contains(index)).collect();
    NodeIdentityMatchResult { matched, unmatched_template, unmatched_destination }
}

pub fn match_owner_paths<T, D>(
    template: &[T],
    destination: &[D],
    template_path: impl for<'a> Fn(&'a T) -> &'a str,
    destination_path: impl for<'a> Fn(&'a D) -> &'a str,
) -> NodeIdentityMatchResult {
    let template = template
        .iter()
        .map(|owner| NodeIdentity::new(Some(NodeSignature::owner_path(template_path(owner)))))
        .collect::<Vec<_>>();
    let destination = destination
        .iter()
        .map(|owner| NodeIdentity::new(Some(NodeSignature::owner_path(destination_path(owner)))))
        .collect::<Vec<_>>();
    match_node_identities(&template, &destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_duplicate_signatures_by_occurrence_without_reusing_nodes() {
        let signature = || NodeIdentity::new(Some(NodeSignature::owner_path("/same")));
        let result = match_node_identities(
            &[signature(), signature(), NodeIdentity::new(Some(NodeSignature::owner_path("/new")))],
            &[signature(), signature(), NodeIdentity::new(Some(NodeSignature::owner_path("/old")))],
        );

        assert_eq!(
            result
                .matched
                .iter()
                .map(|entry| (entry.template_index, entry.destination_index))
                .collect::<Vec<_>>(),
            [(0, 0), (1, 1)]
        );
        assert_eq!(result.unmatched_template, [2]);
        assert_eq!(result.unmatched_destination, [2]);
    }

    #[test]
    fn uses_content_identity_only_when_primary_signatures_do_not_match() {
        let shared_content = NodeSignature::content_identity("same body");
        let template = NodeIdentity::new(Some(NodeSignature::owner_path("/old")))
            .with_alias(shared_content.clone());
        let destination =
            NodeIdentity::new(Some(NodeSignature::owner_path("/new"))).with_alias(shared_content);
        let result = match_node_identities(&[template], &[destination]);

        assert_eq!(result.matched.len(), 1);
        assert_eq!(result.matched[0].signature.0[0], "content_identity");
    }
}
