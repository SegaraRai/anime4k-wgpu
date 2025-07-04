//! Physical texture allocation and lifetime management
//!
//! This module handles the allocation of physical GPU textures from logical texture
//! descriptions, optimizing memory usage through texture reuse when lifetimes don't overlap.

use super::ScaleFactor;
use serde::Serialize;
use std::collections::HashMap;

/// Represents the lifetime of a logical texture in the pipeline
#[derive(Debug, Clone)]
pub struct TextureLifetime {
    /// Logical identifier for this texture
    pub logical_id: String,
    /// Number of color components (1, 2, 3, or 4)
    pub components: u32,
    /// Scale factors relative to input dimensions
    pub scale_factor: (ScaleFactor, ScaleFactor),
    /// Pass index where this texture is first created
    pub created_at: usize,
    /// Pass index where this texture is last used
    pub last_used_at: usize,
}

/// A physical texture that will be allocated in the GPU
#[derive(Debug, Clone, Serialize)]
pub struct PhysicalTexture {
    pub id: u32,
    pub components: u32,
    pub scale_factor: (ScaleFactor, ScaleFactor),
    pub is_source: bool,
}

/// Assigns physical textures to logical texture lifetimes, optimizing memory usage
/// by reusing compatible physical textures when their lifetimes don't overlap.
///
/// # Arguments
/// * `texture_lifetimes` - Slice of texture lifetimes to assign physical textures to
///
/// # Returns
/// A tuple containing:
/// * `Vec<PhysicalTexture>` - Vector of unique physical textures (no duplicates)
/// * `HashMap<String, u32>` - Mapping from logical texture names to physical texture IDs
pub fn assign_physical_textures(texture_lifetimes: &[TextureLifetime]) -> (Vec<PhysicalTexture>, HashMap<String, u32>) {
    let mut physical_textures = Vec::new();
    let mut texture_assignments = HashMap::new();
    let mut physical_texture_slots: Vec<Option<TextureLifetime>> = Vec::new();

    // Add SOURCE texture
    let source_id = u32::MAX;
    physical_textures.push(PhysicalTexture {
        id: source_id,
        components: 4, // Assume RGBA for source
        scale_factor: (ScaleFactor::new(1, 1), ScaleFactor::new(1, 1)),
        is_source: true,
    });
    texture_assignments.insert("SOURCE".to_string(), source_id);

    for lifetime in texture_lifetimes {
        let mut assigned_physical_id = None;
        let mut is_reused = false;

        // Try to reuse an existing physical texture
        for (physical_id, slot) in physical_texture_slots.iter_mut().enumerate() {
            if let Some(existing) = slot {
                // Check if we can reuse this texture:
                // 1. Previous texture's lifetime has ended
                // 2. Same number of components
                // 3. Same scale factor
                if existing.last_used_at < lifetime.created_at && existing.components == lifetime.components && existing.scale_factor == lifetime.scale_factor {
                    // Reuse this physical texture
                    assigned_physical_id = Some(physical_id as u32);
                    *slot = Some(lifetime.clone());
                    is_reused = true;
                    break;
                }
            } else {
                // Empty slot, use it
                assigned_physical_id = Some(physical_id as u32);
                *slot = Some(lifetime.clone());
                break;
            }
        }

        // If no existing texture could be reused, create a new one
        let physical_id = if let Some(id) = assigned_physical_id {
            id
        } else {
            let id = physical_texture_slots.len() as u32;
            physical_texture_slots.push(Some(lifetime.clone()));
            id
        };

        // Only create a new physical texture entry if not reusing an existing one
        if !is_reused {
            physical_textures.push(PhysicalTexture {
                id: physical_id,
                components: lifetime.components,
                scale_factor: lifetime.scale_factor,
                is_source: false,
            });
        }

        // Record assignment
        texture_assignments.insert(lifetime.logical_id.clone(), physical_id);
    }

    (physical_textures, texture_assignments)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper function to validate that physical textures don't have duplicate IDs
    fn assert_no_duplicate_physical_texture_ids(physical_textures: &[PhysicalTexture]) {
        let mut id_counts = std::collections::HashMap::new();

        for texture in physical_textures {
            let count = id_counts.entry(texture.id).or_insert(0);
            *count += 1;
        }

        // Find any IDs that appear more than once
        let duplicates: Vec<_> = id_counts.iter().filter(|(_, count)| **count > 1).collect();

        if !duplicates.is_empty() {
            panic!("Found duplicate physical texture IDs: {duplicates:?}. Each physical texture ID should appear exactly once in the physical_textures vector.");
        }
    }

    #[test]
    fn test_empty_texture_lifetimes() {
        let (physical_textures, assignments) = assign_physical_textures(&[]);

        // Conformance check: no duplicate physical texture IDs
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Should only have the SOURCE texture
        assert_eq!(physical_textures.len(), 1);
        assert_eq!(assignments.len(), 1);

        let source_texture = &physical_textures[0];
        assert_eq!(source_texture.id, u32::MAX);
        assert_eq!(source_texture.components, 4);
        assert_eq!(source_texture.scale_factor, (ScaleFactor::new(1, 1), ScaleFactor::new(1, 1)));
        assert!(source_texture.is_source);

        assert_eq!(assignments.get("SOURCE"), Some(&u32::MAX));
    }

    #[test]
    fn test_single_texture_lifetime() {
        let lifetime = TextureLifetime {
            logical_id: "TEMP1".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 0,
            last_used_at: 5,
        };

        let (physical_textures, assignments) = assign_physical_textures(&[lifetime]);

        // Conformance check: no duplicate physical texture IDs
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Should have SOURCE + one physical texture
        assert_eq!(physical_textures.len(), 2);
        assert_eq!(assignments.len(), 2);

        // Check SOURCE texture
        let source_texture = physical_textures.iter().find(|t| t.is_source).unwrap();
        assert_eq!(source_texture.id, u32::MAX);
        assert!(source_texture.is_source);

        // Check the allocated texture
        let allocated_texture = physical_textures.iter().find(|t| !t.is_source).unwrap();
        assert_eq!(allocated_texture.id, 0);
        assert_eq!(allocated_texture.components, 4);
        assert_eq!(allocated_texture.scale_factor, (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)));
        assert!(!allocated_texture.is_source);

        assert_eq!(assignments.get("TEMP1"), Some(&0));
        assert_eq!(assignments.get("SOURCE"), Some(&u32::MAX));
    }

    #[test]
    fn test_texture_reuse_compatible() {
        let lifetime1 = TextureLifetime {
            logical_id: "TEMP1".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 0,
            last_used_at: 3,
        };

        let lifetime2 = TextureLifetime {
            logical_id: "TEMP2".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 4, // Starts after lifetime1 ends
            last_used_at: 7,
        };

        let (physical_textures, assignments) = assign_physical_textures(&[lifetime1, lifetime2]);

        // Conformance check: no duplicate physical texture IDs
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Should have SOURCE + one reused physical texture = 2 total
        assert_eq!(physical_textures.len(), 2);

        // Both logical textures should be assigned to the same physical ID (texture reuse)
        let temp1_id = assignments.get("TEMP1").unwrap();
        let temp2_id = assignments.get("TEMP2").unwrap();

        // This should be the same ID since textures are compatible and non-overlapping
        assert_eq!(temp1_id, temp2_id);
        assert_eq!(*temp1_id, 0); // Both should use physical texture ID 0

        // Verify we have exactly one non-source texture with the correct properties
        let non_source_textures: Vec<_> = physical_textures.iter().filter(|t| !t.is_source).collect();
        assert_eq!(non_source_textures.len(), 1);

        let reused_texture = non_source_textures[0];
        assert_eq!(reused_texture.id, 0);
        assert_eq!(reused_texture.components, 4);
        assert_eq!(reused_texture.scale_factor, (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)));
        assert!(!reused_texture.is_source);
    }

    #[test]
    fn test_texture_reuse_incompatible_components() {
        let lifetime1 = TextureLifetime {
            logical_id: "TEMP1".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 0,
            last_used_at: 3,
        };

        let lifetime2 = TextureLifetime {
            logical_id: "TEMP2".to_string(),
            components: 1, // Different component count
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 4,
            last_used_at: 7,
        };

        let (physical_textures, assignments) = assign_physical_textures(&[lifetime1, lifetime2]);

        // Conformance check: no duplicate physical texture IDs
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Should have SOURCE + two separate physical textures (different components)
        assert_eq!(physical_textures.len(), 3);

        let temp1_id = assignments.get("TEMP1").unwrap();
        let temp2_id = assignments.get("TEMP2").unwrap();

        // Should be different physical textures due to incompatible components
        assert_ne!(temp1_id, temp2_id);

        // Verify the component counts are correct
        let temp1_texture = physical_textures.iter().find(|t| t.id == *temp1_id).unwrap();
        let temp2_texture = physical_textures.iter().find(|t| t.id == *temp2_id).unwrap();

        assert_eq!(temp1_texture.components, 4);
        assert_eq!(temp2_texture.components, 1);
    }

    #[test]
    fn test_texture_reuse_incompatible_scale_factor() {
        let lifetime1 = TextureLifetime {
            logical_id: "TEMP1".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 0,
            last_used_at: 3,
        };

        let lifetime2 = TextureLifetime {
            logical_id: "TEMP2".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(1, 1), ScaleFactor::new(1, 1)), // Different scale factor
            created_at: 4,
            last_used_at: 7,
        };

        let (physical_textures, assignments) = assign_physical_textures(&[lifetime1, lifetime2]);

        // Conformance check: no duplicate physical texture IDs
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Should have SOURCE + two separate physical textures (different scale factors)
        assert_eq!(physical_textures.len(), 3);

        let temp1_id = assignments.get("TEMP1").unwrap();
        let temp2_id = assignments.get("TEMP2").unwrap();

        // Should be different physical textures due to incompatible scale factors
        assert_ne!(temp1_id, temp2_id);

        // Verify the scale factors are correct
        let temp1_texture = physical_textures.iter().find(|t| t.id == *temp1_id).unwrap();
        let temp2_texture = physical_textures.iter().find(|t| t.id == *temp2_id).unwrap();

        assert_eq!(temp1_texture.scale_factor, (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)));
        assert_eq!(temp2_texture.scale_factor, (ScaleFactor::new(1, 1), ScaleFactor::new(1, 1)));
    }

    #[test]
    fn test_overlapping_lifetimes() {
        let lifetime1 = TextureLifetime {
            logical_id: "TEMP1".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 0,
            last_used_at: 5,
        };

        let lifetime2 = TextureLifetime {
            logical_id: "TEMP2".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 3, // Overlaps with lifetime1
            last_used_at: 7,
        };

        let (physical_textures, assignments) = assign_physical_textures(&[lifetime1, lifetime2]);

        // Conformance check: no duplicate physical texture IDs
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Should have SOURCE + two separate physical textures (overlapping lifetimes)
        assert_eq!(physical_textures.len(), 3);

        let temp1_id = assignments.get("TEMP1").unwrap();
        let temp2_id = assignments.get("TEMP2").unwrap();

        // Should be different physical textures due to overlapping lifetimes
        assert_ne!(temp1_id, temp2_id);
    }

    #[test]
    fn test_multiple_texture_chain() {
        let lifetime1 = TextureLifetime {
            logical_id: "TEMP1".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 0,
            last_used_at: 2,
        };

        let lifetime2 = TextureLifetime {
            logical_id: "TEMP2".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 3,
            last_used_at: 5,
        };

        let lifetime3 = TextureLifetime {
            logical_id: "TEMP3".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 6,
            last_used_at: 8,
        };

        let (physical_textures, assignments) = assign_physical_textures(&[lifetime1, lifetime2, lifetime3]);

        // Conformance check: no duplicate physical texture IDs
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // These textures can all reuse the same physical texture
        // Should have SOURCE + 1 reused texture = 2 total
        assert_eq!(physical_textures.len(), 2);

        // All three logical textures should be assigned to the same physical ID
        let temp1_id = assignments.get("TEMP1").unwrap();
        let temp2_id = assignments.get("TEMP2").unwrap();
        let temp3_id = assignments.get("TEMP3").unwrap();

        assert_eq!(temp1_id, temp2_id);
        assert_eq!(temp2_id, temp3_id);
        assert_eq!(*temp1_id, 0); // All should use physical texture ID 0

        // Verify all assignments exist
        assert!(assignments.contains_key("TEMP1"));
        assert!(assignments.contains_key("TEMP2"));
        assert!(assignments.contains_key("TEMP3"));
        assert!(assignments.contains_key("SOURCE"));

        // Should have exactly one non-source texture
        let non_source_textures: Vec<_> = physical_textures.iter().filter(|t| !t.is_source).collect();
        assert_eq!(non_source_textures.len(), 1);

        let reused_texture = non_source_textures[0];
        assert_eq!(reused_texture.id, 0);
        assert_eq!(reused_texture.components, 4);
        assert_eq!(reused_texture.scale_factor, (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)));
        assert!(!reused_texture.is_source);
    }

    #[test]
    fn test_source_texture_properties() {
        let lifetime = TextureLifetime {
            logical_id: "TEMP1".to_string(),
            components: 1,
            scale_factor: (ScaleFactor::new(1, 2), ScaleFactor::new(1, 2)),
            created_at: 0,
            last_used_at: 5,
        };

        let (physical_textures, assignments) = assign_physical_textures(&[lifetime]);

        // Conformance check: no duplicate physical texture IDs
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Find SOURCE texture
        let source_texture = physical_textures.iter().find(|t| t.is_source).unwrap();

        // SOURCE should always have these properties regardless of other textures
        assert_eq!(source_texture.id, u32::MAX);
        assert_eq!(source_texture.components, 4); // Always RGBA for source
        assert_eq!(source_texture.scale_factor, (ScaleFactor::new(1, 1), ScaleFactor::new(1, 1))); // Always 1x1 scale
        assert!(source_texture.is_source);

        assert_eq!(assignments.get("SOURCE"), Some(&u32::MAX));
    }

    #[test]
    fn test_physical_texture_ids_are_sequential() {
        let lifetimes = &[
            TextureLifetime {
                logical_id: "TEMP1".to_string(),
                components: 4,
                scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
                created_at: 0,
                last_used_at: 1,
            },
            TextureLifetime {
                logical_id: "TEMP2".to_string(),
                components: 1,
                scale_factor: (ScaleFactor::new(1, 1), ScaleFactor::new(1, 1)),
                created_at: 0,
                last_used_at: 1,
            },
            TextureLifetime {
                logical_id: "TEMP3".to_string(),
                components: 2,
                scale_factor: (ScaleFactor::new(4, 1), ScaleFactor::new(4, 1)),
                created_at: 0,
                last_used_at: 1,
            },
        ];

        let (physical_textures, assignments) = assign_physical_textures(lifetimes);

        // Conformance check: no duplicate physical texture IDs
        // These textures are all incompatible (different properties), so should be unique
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Should have SOURCE + 3 regular textures
        assert_eq!(physical_textures.len(), 4);

        // Non-source texture IDs should be 0, 1, 2
        let mut non_source_ids: Vec<u32> = physical_textures.iter().filter(|t| !t.is_source).map(|t| t.id).collect();
        non_source_ids.sort();

        assert_eq!(non_source_ids, vec![0, 1, 2]);

        // All logical textures should be assigned
        assert!(assignments.contains_key("TEMP1"));
        assert!(assignments.contains_key("TEMP2"));
        assert!(assignments.contains_key("TEMP3"));
        assert!(assignments.contains_key("SOURCE"));
    }

    #[test]
    fn test_edge_case_same_creation_and_usage_time() {
        let lifetime1 = TextureLifetime {
            logical_id: "TEMP1".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 5,
            last_used_at: 5, // Same time
        };

        let lifetime2 = TextureLifetime {
            logical_id: "TEMP2".to_string(),
            components: 4,
            scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
            created_at: 5, // Same creation time
            last_used_at: 5,
        };

        let (physical_textures, assignments) = assign_physical_textures(&[lifetime1, lifetime2]);

        // Conformance check: no duplicate physical texture IDs
        // These textures have overlapping lifetimes (same time), so should get separate physical textures
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Should handle this edge case without crashing
        assert!(physical_textures.len() >= 2); // At least SOURCE + one texture
        assert_eq!(assignments.len(), 3); // SOURCE + TEMP1 + TEMP2

        assert!(assignments.contains_key("TEMP1"));
        assert!(assignments.contains_key("TEMP2"));
        assert!(assignments.contains_key("SOURCE"));
    }

    #[test]
    fn test_complex_scenario_mixed_reuse_and_conflicts() {
        // This test covers a complex scenario with multiple types of texture relationships:
        // - Some textures can be reused
        // - Some textures conflict due to overlapping lifetimes
        // - Some textures conflict due to incompatible properties

        let lifetimes = &[
            // Group A: Compatible textures that can be reused sequentially
            TextureLifetime {
                logical_id: "TEMP1".to_string(),
                components: 4,
                scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
                created_at: 0,
                last_used_at: 2,
            },
            TextureLifetime {
                logical_id: "TEMP2".to_string(),
                components: 4,
                scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
                created_at: 3, // Can reuse TEMP1's physical texture
                last_used_at: 5,
            },
            // Group B: Overlapping with Group A (needs separate physical texture)
            TextureLifetime {
                logical_id: "TEMP3".to_string(),
                components: 4,
                scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
                created_at: 1,   // Overlaps with TEMP1
                last_used_at: 4, // Also overlaps with TEMP2
            },
            // Group C: Different properties (needs separate physical texture)
            TextureLifetime {
                logical_id: "TEMP4".to_string(),
                components: 1, // Different component count
                scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
                created_at: 6,
                last_used_at: 8,
            },
            // Group D: Can reuse Group A's texture after all others are done
            TextureLifetime {
                logical_id: "TEMP5".to_string(),
                components: 4,
                scale_factor: (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)),
                created_at: 9, // After all previous compatible textures
                last_used_at: 11,
            },
        ];

        let (physical_textures, assignments) = assign_physical_textures(lifetimes);

        // Conformance check: no duplicate physical texture IDs
        assert_no_duplicate_physical_texture_ids(&physical_textures);

        // Expected optimal allocation:
        // - SOURCE texture (id: u32::MAX)
        // - Physical texture 0: Used by TEMP1, TEMP2, TEMP5 (reused)
        // - Physical texture 1: Used by TEMP3 (overlapping lifetimes)
        // - Physical texture 2: Used by TEMP4 (different properties)
        // Total: 4 physical textures
        assert_eq!(physical_textures.len(), 4);

        // Verify assignments
        assert_eq!(assignments.get("SOURCE"), Some(&u32::MAX));

        // Group A textures should share the same physical ID
        let temp1_id = *assignments.get("TEMP1").unwrap();
        let temp2_id = *assignments.get("TEMP2").unwrap();
        let temp5_id = *assignments.get("TEMP5").unwrap();
        assert_eq!(temp1_id, temp2_id);
        assert_eq!(temp2_id, temp5_id);

        // TEMP3 should have a different ID (overlapping lifetimes)
        let temp3_id = *assignments.get("TEMP3").unwrap();
        assert_ne!(temp1_id, temp3_id);

        // TEMP4 should have a different ID (different properties)
        let temp4_id = *assignments.get("TEMP4").unwrap();
        assert_ne!(temp1_id, temp4_id);
        assert_ne!(temp3_id, temp4_id);

        // Verify each physical texture has the correct properties
        for texture in &physical_textures {
            if texture.is_source {
                assert_eq!(texture.id, u32::MAX);
                assert_eq!(texture.components, 4);
                assert_eq!(texture.scale_factor, (ScaleFactor::new(1, 1), ScaleFactor::new(1, 1)));
            } else {
                // All non-source textures should have the expected scale factor
                assert_eq!(texture.scale_factor, (ScaleFactor::new(2, 1), ScaleFactor::new(2, 1)));
                assert!(!texture.is_source);

                // components should be either 4 (for most textures) or 1 (for TEMP4)
                assert!(texture.components == 4 || texture.components == 1);
            }
        }

        // Verify exactly one texture has 1 component (TEMP4's physical texture)
        let single_component_textures: Vec<_> = physical_textures.iter().filter(|t| !t.is_source && t.components == 1).collect();
        assert_eq!(single_component_textures.len(), 1);
        assert_eq!(single_component_textures[0].id, temp4_id);
    }
}
