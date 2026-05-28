//! UI types — Pen format + S.DEF extensions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// User Interface root
// ============================================================================

/// User interface — three layers: design system, Pen document, abstract screens.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserInterface {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design_system: Option<UIDesignSystem>,

    /// Pen-compatible visual document with S.DEF extensions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<UIDocument>,

    /// Abstract screen definitions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screens: Option<Vec<UIScreen>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub navigation: Option<UINavigation>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub responsive_design: Option<Vec<ResponsiveBreakpoint>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub component_taxonomy: Option<Vec<UIComponentType>>,
}

// ============================================================================
// Layer 1: Design System (Tokens & Themes)
// ============================================================================

/// Design tokens: colors, typography, spacing, shadows, themes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UIDesignSystem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub typography: Option<UIDesignTypography>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub spacing: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_radius: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadows: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub motion: Option<UIDesignMotion>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub themes: Option<Vec<UIDesignTheme>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UIDesignTypography {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_families: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_sizes: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weights: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_heights: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UIDesignMotion {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub durations: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub easings: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIDesignTheme {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub overrides: Option<HashMap<String, String>>,
}

// ============================================================================
// Layer 2: Pen-compatible Visual Document
// ============================================================================

/// Pen-compatible UI document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIDocument {
    /// Pen format version.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, UIVariable>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub themes: Option<HashMap<String, Vec<String>>>,

    pub children: Vec<UINode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIVariable {
    pub type_: String,
    pub value: serde_json::Value,
}

/// Base element shared by all Pen visual nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIBaseElement {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    pub type_: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<serde_json::Value>,

    #[serde(default)]
    pub reusable: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<HashMap<String, String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub opacity: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<f64>,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stroke: Option<UIStroke>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<UINode>>,

    // ---- S.DEF semantic extensions ----
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdef_bindings: Option<Vec<UIDataBinding>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdef_behaviors: Option<Vec<UIBehavior>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdef_states: Option<Vec<UIVisualState>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdef_accessibility: Option<UIAccessibility>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdef_test_hook: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdef_navigation: Option<UINavTarget>,
}

fn default_true() -> bool { true }

/// Stroke definition.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UIStroke {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thickness: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill: Option<serde_json::Value>,
}

// ---- S.DEF semantic extensions ----

/// Data binding — links UI element to a data entity field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIDataBinding {
    pub entity: String,
    pub field: String,

    /// "one_way" | "two_way".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
}

/// Interaction behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIBehavior {
    /// Trigger event (click, submit, focus, enter).
    pub on: String,
    pub action: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_success: Option<UIBehaviorOutcome>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_error: Option<UIBehaviorOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIBehaviorOutcome {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Visual state condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIVisualState {
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<UIVisualStateCondition>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIVisualStateCondition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub operator: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

/// Accessibility metadata.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UIAccessibility {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria_label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub aria_role: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub keyboard_shortcut: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_reader_text: Option<String>,
}

/// Navigation target.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UINavTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_screen: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, String>>,
}

// ---- Pen element types ----

/// Frame (container with flexbox layout).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIFrame {
    #[serde(flatten)]
    pub base: UIBaseElement,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub justify_content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_items: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f64>,

    #[serde(default)]
    pub clip: bool,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub slot: Option<serde_json::Value>,
}

/// Text element.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIText {
    #[serde(flatten)]
    pub base: UIBaseElement,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_size: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_weight: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_align: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_height: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_decoration: Option<String>,
}

/// Union type for all visual nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UINode {
    Frame(UIFrame),
    Text(UIText),
    Rectangle(UIRectangle),
    Ellipse(UIEllipse),
    Path(UIPath),
    Ref(UIRef),
    IconFont(UIIconFont),
    Base(UIBaseElement),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIRectangle {
    #[serde(flatten)]
    pub base: UIBaseElement,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub corner_radius: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIEllipse {
    #[serde(flatten)]
    pub base: UIBaseElement,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_radius: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIPath {
    #[serde(flatten)]
    pub base: UIBaseElement,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub fill_rule: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIRef {
    #[serde(flatten)]
    pub base: UIBaseElement,

    pub ref_: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub descendants: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIIconFont {
    #[serde(flatten)]
    pub base: UIBaseElement,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_font_family: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
}

// ============================================================================
// Layer 3: Abstract Screens
// ============================================================================

/// Abstract screen description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIScreen {
    pub id: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Vec<UIComponent>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<UIState>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub interactions: Option<Vec<UIInteraction>>,
}

/// UI component in an abstract screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIComponent {
    pub name: String,
    pub type_: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub states: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<UIComponent>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub behaviors: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_to: Option<String>,
}

/// Screen-level state.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UIState {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub global: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub local: Option<Vec<String>>,
}

/// User interaction on a screen.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIInteraction {
    pub trigger: String,
    pub action: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_success: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_error: Option<String>,
}

// ============================================================================
// Navigation & Responsive
// ============================================================================

/// Application navigation structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UINavigation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub nodes: Option<Vec<UINavNode>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transitions: Option<Vec<UINavTransition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UINavNode {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub route: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<UINavNode>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UINavTransition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub animation: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
}

/// Responsive breakpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsiveBreakpoint {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_width: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,

    pub description: String,
}

/// UI component taxonomy entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UIComponentType {
    pub component_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_requirements: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub interaction_rules: Option<Vec<String>>,
}
