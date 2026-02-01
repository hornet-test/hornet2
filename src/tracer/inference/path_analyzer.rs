//! Path parameter detection and route inference.
//!
//! Analyzes observed paths to detect path parameters and infer route patterns.

use std::collections::{HashMap, HashSet};

/// Analyzes paths to detect parameters and infer route patterns.
pub struct PathAnalyzer {
    /// Minimum number of unique values to consider a segment as a parameter
    min_unique_values: usize,
    /// Maximum ratio of unique values to total occurrences for parameter detection
    max_unique_ratio: f64,
    /// Known parameter patterns (regex-like)
    known_patterns: Vec<KnownPattern>,
}

/// A known pattern for detecting parameters
#[derive(Clone)]
struct KnownPattern {
    #[allow(dead_code)]
    name: String,
    matcher: fn(&str) -> bool,
}

/// Result of path analysis
#[derive(Debug, Clone)]
pub struct PathAnalysisResult {
    /// The inferred route pattern (e.g., "/users/{userId}")
    pub route: String,
    /// Detected parameters with their names and example values
    pub parameters: Vec<PathParameter>,
    /// Number of paths that matched this route
    pub occurrence_count: usize,
}

/// A detected path parameter
#[derive(Debug, Clone)]
pub struct PathParameter {
    /// Parameter name (e.g., "userId")
    pub name: String,
    /// Position in the path (0-indexed)
    pub position: usize,
    /// Example values observed
    pub example_values: Vec<String>,
    /// Inferred type
    pub param_type: ParameterType,
}

/// Inferred parameter type
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterType {
    /// UUID format
    Uuid,
    /// Integer
    Integer,
    /// Alphanumeric slug
    Slug,
    /// Generic string
    String,
}

impl PathAnalyzer {
    /// Create a new path analyzer with default settings
    pub fn new() -> Self {
        Self {
            min_unique_values: 2,
            max_unique_ratio: 0.8,
            known_patterns: Self::default_patterns(),
        }
    }

    /// Set minimum unique values threshold
    pub fn with_min_unique_values(mut self, min: usize) -> Self {
        self.min_unique_values = min;
        self
    }

    /// Set maximum unique ratio threshold
    pub fn with_max_unique_ratio(mut self, ratio: f64) -> Self {
        self.max_unique_ratio = ratio;
        self
    }

    fn default_patterns() -> Vec<KnownPattern> {
        vec![
            KnownPattern {
                name: "uuid".to_string(),
                matcher: |s| {
                    s.len() == 36
                        && s.chars().enumerate().all(|(i, c)| match i {
                            8 | 13 | 18 | 23 => c == '-',
                            _ => c.is_ascii_hexdigit(),
                        })
                },
            },
            KnownPattern {
                name: "integer".to_string(),
                matcher: |s| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit()),
            },
            KnownPattern {
                name: "slug".to_string(),
                matcher: |s| {
                    !s.is_empty()
                        && s.chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                },
            },
        ]
    }

    /// Analyze a collection of paths and return inferred routes
    pub fn analyze(&self, paths: &[(String, String)]) -> Vec<PathAnalysisResult> {
        // Group paths by method
        let mut method_paths: HashMap<String, Vec<&str>> = HashMap::new();
        for (method, path) in paths {
            method_paths
                .entry(method.clone())
                .or_default()
                .push(path.as_str());
        }

        let mut results = Vec::new();

        for (method, paths) in method_paths {
            let method_results = self.analyze_paths(&paths);
            for mut result in method_results {
                // Prefix method info to route for uniqueness tracking
                result.route = format!("{} {}", method, result.route);
                results.push(result);
            }
        }

        // Sort by occurrence count (descending)
        results.sort_by(|a, b| b.occurrence_count.cmp(&a.occurrence_count));

        results
    }

    /// Analyze paths for a single method
    fn analyze_paths(&self, paths: &[&str]) -> Vec<PathAnalysisResult> {
        // Group paths by segment count
        let mut by_length: HashMap<usize, Vec<Vec<&str>>> = HashMap::new();
        for path in paths {
            let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
            by_length.entry(segments.len()).or_default().push(segments);
        }

        let mut results = Vec::new();

        for (_, segment_groups) in by_length {
            if segment_groups.is_empty() {
                continue;
            }

            let segment_count = segment_groups[0].len();
            let total_paths = segment_groups.len();

            // Analyze each segment position
            let mut param_positions: Vec<Option<PathParameter>> = vec![None; segment_count];
            let mut param_index = 0usize;

            for pos in 0..segment_count {
                let values: Vec<&str> = segment_groups.iter().map(|s| s[pos]).collect();
                let unique_values: HashSet<&str> = values.iter().copied().collect();

                // Check if this position should be a parameter
                if self.should_be_parameter(&values, &unique_values, total_paths) {
                    let param_type = self.infer_type(&unique_values);
                    let name = self.infer_name(param_index, &unique_values, &param_type);

                    param_positions[pos] = Some(PathParameter {
                        name,
                        position: pos,
                        example_values: unique_values
                            .into_iter()
                            .take(5)
                            .map(String::from)
                            .collect(),
                        param_type,
                    });
                    param_index += 1;
                }
            }

            // Build route pattern
            let route_segments: Vec<String> = (0..segment_count)
                .map(|pos| {
                    if let Some(ref param) = param_positions[pos] {
                        format!("{{{}}}", param.name)
                    } else {
                        // Use the most common value for static segments
                        segment_groups[0][pos].to_string()
                    }
                })
                .collect();

            let route = format!("/{}", route_segments.join("/"));
            let parameters: Vec<PathParameter> = param_positions.into_iter().flatten().collect();

            results.push(PathAnalysisResult {
                route,
                parameters,
                occurrence_count: total_paths,
            });
        }

        // Merge similar routes
        self.merge_similar_routes(results)
    }

    fn should_be_parameter(
        &self,
        _values: &[&str],
        unique_values: &HashSet<&str>,
        total_paths: usize,
    ) -> bool {
        let unique_count = unique_values.len();

        // If all values are the same, it's not a parameter
        if unique_count == 1 {
            return false;
        }

        // If we have enough unique values
        if unique_count >= self.min_unique_values {
            // Check if all values match known patterns (UUID, integer, slug)
            let all_match_pattern = unique_values
                .iter()
                .all(|v| self.known_patterns.iter().any(|p| (p.matcher)(v)));

            // If all values match a known parameter pattern, it's definitely a parameter
            if all_match_pattern {
                return true;
            }

            // Otherwise, use ratio heuristic
            let ratio = unique_count as f64 / total_paths as f64;
            if ratio <= self.max_unique_ratio && ratio >= 0.3 {
                return true;
            }
        }

        false
    }

    fn infer_type(&self, values: &HashSet<&str>) -> ParameterType {
        // Check UUID first (most specific)
        if values.iter().all(|v| (self.known_patterns[0].matcher)(v)) {
            return ParameterType::Uuid;
        }

        // Check integer
        if values.iter().all(|v| (self.known_patterns[1].matcher)(v)) {
            return ParameterType::Integer;
        }

        // Check slug
        if values.iter().all(|v| (self.known_patterns[2].matcher)(v)) {
            return ParameterType::Slug;
        }

        ParameterType::String
    }

    fn infer_name(
        &self,
        param_index: usize,
        _values: &HashSet<&str>,
        param_type: &ParameterType,
    ) -> String {
        // Common naming patterns based on parameter index (0-based) and type
        // First parameter gets no suffix, subsequent ones get 2, 3, etc.
        let suffix = if param_index > 0 {
            (param_index + 1).to_string()
        } else {
            String::new()
        };
        match param_type {
            ParameterType::Uuid => format!("id{}", suffix),
            ParameterType::Integer => format!("id{}", suffix),
            ParameterType::Slug => format!("slug{}", suffix),
            ParameterType::String => format!("param{}", suffix),
        }
    }

    fn merge_similar_routes(&self, routes: Vec<PathAnalysisResult>) -> Vec<PathAnalysisResult> {
        // Group routes by their static segments pattern
        let mut groups: HashMap<String, Vec<PathAnalysisResult>> = HashMap::new();

        for route in routes {
            let key = self.route_signature(&route.route);
            groups.entry(key).or_default().push(route);
        }

        groups
            .into_values()
            .map(|mut group| {
                if group.len() == 1 {
                    group.pop().unwrap()
                } else {
                    // Merge routes with the same signature
                    let total_count: usize = group.iter().map(|r| r.occurrence_count).sum();
                    let mut merged = group.pop().unwrap();
                    merged.occurrence_count = total_count;

                    // Merge example values
                    for other in group {
                        for (i, param) in other.parameters.into_iter().enumerate() {
                            if let Some(existing) = merged.parameters.get_mut(i) {
                                existing.example_values.extend(param.example_values);
                                existing.example_values.truncate(10);
                            }
                        }
                    }

                    merged
                }
            })
            .collect()
    }

    fn route_signature(&self, route: &str) -> String {
        route
            .split('/')
            .map(|seg| {
                if seg.starts_with('{') && seg.ends_with('}') {
                    "{*}".to_string()
                } else {
                    seg.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("/")
    }
}

impl Default for PathAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_integer_parameter() {
        let analyzer = PathAnalyzer::new();
        let paths = vec![
            ("GET".to_string(), "/users/1".to_string()),
            ("GET".to_string(), "/users/2".to_string()),
            ("GET".to_string(), "/users/3".to_string()),
            ("GET".to_string(), "/users/42".to_string()),
        ];

        let results = analyzer.analyze(&paths);
        assert_eq!(results.len(), 1);
        assert!(results[0].route.contains("{id}"));
        assert_eq!(results[0].parameters.len(), 1);
        assert_eq!(results[0].parameters[0].param_type, ParameterType::Integer);
    }

    #[test]
    fn test_detect_uuid_parameter() {
        let analyzer = PathAnalyzer::new();
        let paths = vec![
            (
                "GET".to_string(),
                "/users/550e8400-e29b-41d4-a716-446655440000".to_string(),
            ),
            (
                "GET".to_string(),
                "/users/6ba7b810-9dad-11d1-80b4-00c04fd430c8".to_string(),
            ),
        ];

        let results = analyzer.analyze(&paths);
        assert_eq!(results.len(), 1);
        assert!(results[0].route.contains("{id}"));
        assert_eq!(results[0].parameters[0].param_type, ParameterType::Uuid);
    }

    #[test]
    fn test_static_path() {
        let analyzer = PathAnalyzer::new();
        let paths = vec![
            ("GET".to_string(), "/api/health".to_string()),
            ("GET".to_string(), "/api/health".to_string()),
        ];

        let results = analyzer.analyze(&paths);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].route, "GET /api/health");
        assert!(results[0].parameters.is_empty());
    }

    #[test]
    fn test_multiple_parameters() {
        let analyzer = PathAnalyzer::new();
        let paths = vec![
            ("GET".to_string(), "/users/1/posts/100".to_string()),
            ("GET".to_string(), "/users/2/posts/200".to_string()),
            ("GET".to_string(), "/users/3/posts/300".to_string()),
        ];

        let results = analyzer.analyze(&paths);
        assert_eq!(results.len(), 1);
        assert!(results[0].route.contains("{id}"));
        assert!(results[0].route.contains("posts"));
        assert_eq!(results[0].parameters.len(), 2);
    }

    #[test]
    fn test_parameter_type_inference() {
        let values: HashSet<&str> = ["123", "456", "789"].into_iter().collect();
        let analyzer = PathAnalyzer::new();
        assert_eq!(analyzer.infer_type(&values), ParameterType::Integer);

        let values: HashSet<&str> = ["abc-def", "ghi-jkl"].into_iter().collect();
        assert_eq!(analyzer.infer_type(&values), ParameterType::Slug);
    }
}
