//! Changing a node's properties: the tool's edge half.

use schema::WireName;
use serde_json::{json, Value};

use super::args::string;
use super::data;
use super::schema::{handle_property, schema};
use super::{Request, Tool};
use crate::tools::{images, rich_text, style};
use crate::ToolError;

pub struct SetProperties;

impl Tool for SetProperties {
    fn name(&self) -> &'static str {
        "set_properties"
    }
    fn description(&self) -> &'static str {
        "Change a frame, instance or text layer you created, using read_node's names: x, y, width, height, name, \
         visible, clipsContent (clip children to the frame), auto-layout (layoutMode NONE | \
         HORIZONTAL | VERTICAL, itemSpacing, counterAxisSpacing, padding or paddingTop/Right/\
         Bottom/Left, primaryAxisSizingMode and counterAxisSizingMode FIXED | AUTO, \
         primaryAxisAlignItems MIN | CENTER | MAX | SPACE_BETWEEN, counterAxisAlignItems MIN | \
         CENTER | MAX, layoutWrap), a child's layoutGrow (0 | 1), layoutAlign (INHERIT | \
         STRETCH) and layoutPositioning (AUTO | ABSOLUTE), fills and strokes (SOLID paints: \
         { type: SOLID, color: \"#RRGGBB\", opacity?, variable? }, where variable binds the \
         paint to one of the file's local colour variables by name; or { type: GRADIENT_LINEAR \
         | GRADIENT_RADIAL, gradientStops: [{ position: 0-1, color, opacity? }], angle? (linear, \
         degrees: 180 top to bottom, 90 left to right) or gradientTransform? }; or { type: IMAGE, \
         image: \"<PNG or JPEG path>\" or imageHash, scaleMode?: FILL | FIT | STRETCH }), strokeWeight, strokeAlign \
         (INSIDE, CENTER, OUTSIDE), cornerRadius or topLeftRadius/topRightRadius/\
         bottomRightRadius/bottomLeftRadius, effects ({ type: DROP_SHADOW | INNER_SHADOW | \
         LAYER_BLUR | BACKGROUND_BLUR, color?, opacity?, offset?: {x,y}, radius, spread? }) and \
         opacity, and dashPattern (dash then gap lengths, [] for solid). On an instance: x, y, name, \
         visible, size, and its own fills, strokes, strokeWeight, strokeAlign and dashPattern, \
         which it keeps while the rest follows its component. On a text layer: x, y, width (fixes the \
         width and wraps), height (fixes the box), name, visible, characters, fontFamily, fontStyle, \
         fontSize, fills, textAlignHorizontal, lineHeight, letterSpacing, opacity, and as \
         create_text takes them: runs (replacing the characters and their styles), paragraphs, \
         paragraphSpacing, paragraphIndent, textDecoration, textCase, textAlignVertical, \
         textAutoResize. A font, size, colour, spacing, decoration or case set on the layer \
         applies to every character; new characters without runs take the layer's style. A text whose \
         font, size, characters or runs change is estimated again (the result gives its size, \
         provisional) and its auto-layout parent follows. A number that can come from a \
         design token takes { variable: <name> } instead: itemSpacing, counterAxisSpacing, padding \
         and each side, cornerRadius and each corner, width, height, opacity, and a text's \
         fontSize; the value it resolves to is stored beside the binding. explicitVariableModes: \
         { \"Theme\": \"Dark\" } shows a frame and everything in it in that mode. fillStyle, \
         strokeStyle, effectStyle and textStyle apply a style by name (list_styles), null \
         detaches one, and setting fills, strokes, effects or a text's font directly detaches \
         the style it came from. componentPropertyReferences binds a layer inside a component \
         to one of its properties; componentProperties sets an instance's values for them. \
         On a slide (Figma Slides), a frame's properties and: transition { style (NONE, \
         DISSOLVE, SMART_ANIMATE, SLIDE_FROM_, PUSH_FROM_, MOVE_FROM_ LEFT | RIGHT | TOP | \
         BOTTOM, SLIDE_OUT_TO_, MOVE_OUT_TO_ the same), duration 0.01-10 s, curve (EASE_IN, \
         EASE_OUT, EASE_IN_AND_OUT, LINEAR, EASE_IN_BACK, EASE_OUT_BACK, EASE_IN_AND_OUT_BACK, \
         GENTLE), timing { type: ON_CLICK } or { type: AFTER_DELAY, delay } }, as the plugin \
         API's setSlideTransition takes it; objectAnimations [{ node (a layer on the slide), \
         type (FADE, SLIDE_FROM_LEFT | RIGHT | TOP | BOTTOM), phase (IN | OUT), duration }], \
         each played on a click, in order, replacing the slide's; isSkippedSlide; and \
         speakerNotes (text, a paragraph a line). Smart Animate moves the layers two slides \
         share by name. In FigJam: a sticky or shape takes text { characters, fills }, \
         fills, x and y, and a shape also width, height, fontSize and textAlignHorizontal; \
         a table takes cells (rows of texts, the first its header; rows past the last are \
         added) and x, y; a connector takes connectorStart and connectorEnd ({ \
         endpointNodeId, magnet: AUTO | TOP | BOTTOM | LEFT | RIGHT | CENTER } or { \
         position: { x, y } }), connectorStartStrokeCap and connectorEndStrokeCap (NONE, \
         ARROW_LINES, ARROW_EQUILATERAL, TRIANGLE_FILLED, DIAMOND_FILLED, CIRCLE_FILLED…) \
         and connectorLineType (STRAIGHT, ELBOWED, CURVED). To add one, duplicate_node a \
         sticky, shape, table or connector the board has, then set it. Only the \
         properties named change. Nothing changes if any value is refused."
    }
    fn input_schema(&self) -> Value {
        let paint = json!({
            "type": "object",
            "properties": {
                "type": { "enum": style::PAINT_TYPES.map(WireName::as_wire) },
                "image": { "type": "string", "description": "IMAGE: a PNG or JPEG file path, embedded in the file." },
                "imageHash": { "type": "string", "description": "IMAGE: an image the file embeds, by the hash read_node shows." },
                "scaleMode": { "enum": images::SCALE_MODES.map(WireName::as_wire) },
                "gradientStops": { "type": "array", "items": { "type": "object" } },
                "angle": { "type": "number" },
                "gradientTransform": { "type": "array" },
                "color": { "type": "string", "description": "#RRGGBB or #RRGGBBAA" },
                "opacity": { "type": "number", "minimum": 0, "maximum": 1 },
                "variable": { "type": "string", "description": "A local colour variable's name or id." },
            },
            "required": ["type"],
        });
        // A number, or the variable it comes from.
        let either = |number: Value| {
            json!({ "anyOf": [number, { "type": "object", "properties": { "variable": { "type": "string" } }, "required": ["variable"] }] })
        };
        let bindable = json!({
            "opacity": either(json!({ "type": "number", "minimum": 0, "maximum": 1 })),
            "itemSpacing": either(json!({ "type": "number", "minimum": 0 })),
            "counterAxisSpacing": either(json!({ "type": "number", "minimum": 0 })),
            "padding": either(json!({ "type": "number", "minimum": 0 })),
            "paddingTop": either(json!({ "type": "number", "minimum": 0 })),
            "paddingRight": either(json!({ "type": "number", "minimum": 0 })),
            "paddingBottom": either(json!({ "type": "number", "minimum": 0 })),
            "paddingLeft": either(json!({ "type": "number", "minimum": 0 })),
            "explicitVariableModes": { "type": "object", "description": "Collection name → mode name, e.g. { \"Theme\": \"Dark\" }; null clears one." },
            "fillStyle": { "type": ["string", "null"], "description": "A FILL style's name or id; null detaches it." },
            "strokeStyle": { "type": ["string", "null"], "description": "A FILL style for the strokes; null detaches it." },
            "effectStyle": { "type": ["string", "null"] },
            "textStyle": { "type": ["string", "null"], "description": "On a text: a TEXT style's name or id." },
            "componentPropertyReferences": { "type": "object", "description": "On a layer inside a component: { characters?, visible?, mainComponent? }, each a property of the component (add_component_property), or null to unbind." },
            "componentProperties": { "type": "object", "additionalProperties": { "type": ["string", "boolean"] }, "description": "On an instance: property name → value (text, true/false, or a component for a swap)." },
        });
        let mut out = schema(
            json!({
                "handle": handle_property(),
                "id": { "type": "string", "description": "A frame, instance or text layer you created." },
                "properties": {
                    "type": "object",
                    "properties": {
                        "x": { "type": "number" },
                        "y": { "type": "number" },
                        "width": either(json!({ "type": "number", "exclusiveMinimum": 0 })),
                        "height": either(json!({ "type": "number", "exclusiveMinimum": 0 })),
                        "name": { "type": "string" },
                        "visible": { "type": "boolean" },
                        "clipsContent": { "type": "boolean" },
                        "characters": { "type": "string" },
                        "fontFamily": { "type": "string" },
                        "fontStyle": { "type": "string" },
                        "fontSize": either(json!({ "type": "number", "exclusiveMinimum": 0 })),
                        "textAlignHorizontal": { "enum": rich_text::TEXT_ALIGNS_HORIZONTAL.map(WireName::as_wire) },
                        "lineHeight": {},
                        "letterSpacing": {},
                        "fills": { "type": "array", "items": paint },
                        "strokes": { "type": "array", "items": paint },
                        "strokeWeight": { "type": "number", "exclusiveMinimum": 0 },
                        "strokeAlign": { "enum": style::STROKE_ALIGNS.map(WireName::as_wire) },
                        "strokeCap": { "enum": style::STROKE_CAPS.map(WireName::as_wire) },
                        "strokeJoin": { "enum": style::STROKE_JOINS.map(WireName::as_wire) },
                        "cornerRadius": either(json!({ "type": "number", "minimum": 0 })),
                        "topLeftRadius": either(json!({ "type": "number", "minimum": 0 })),
                        "topRightRadius": either(json!({ "type": "number", "minimum": 0 })),
                        "bottomRightRadius": either(json!({ "type": "number", "minimum": 0 })),
                        "bottomLeftRadius": either(json!({ "type": "number", "minimum": 0 })),
                        "effects": { "type": "array", "items": { "type": "object" } },
                    },
                    "additionalProperties": false,
                },
            }),
            &["handle", "id", "properties"],
        );
        // Rich text, apart: one json! this size would exceed the
        // macro's recursion limit.
        let rich = json!({
            "runs": { "type": "array", "items": { "type": "object" }, "description": "On a text: the text as styled pieces, as create_text takes them." },
            "paragraphs": { "type": "array", "items": { "type": "object" }, "description": "On a text: one per line, { listType: NONE | UNORDERED | ORDERED, indentation?: 1 | 2 }." },
            "paragraphSpacing": { "type": "number", "minimum": 0 },
            "paragraphIndent": { "type": "number", "minimum": 0 },
            "textDecoration": { "enum": rich_text::TEXT_DECORATIONS.map(WireName::as_wire) },
            "textCase": { "enum": rich_text::TEXT_CASES.map(WireName::as_wire) },
            "textAlignVertical": { "enum": rich_text::TEXT_ALIGNS_VERTICAL.map(WireName::as_wire) },
            "textAutoResize": { "enum": rich_text::TEXT_AUTO_RESIZES.map(WireName::as_wire) },
        });
        if let (Some(Value::Object(listed)), Value::Object(more), Value::Object(rich)) =
            (out.pointer_mut("/properties/properties/properties"), bindable, rich)
        {
            listed.extend(more);
            listed.extend(rich);
        }
        out
    }

    fn parse(&self, args: &Value) -> Result<Request, ToolError> {
        let handle = string(args, "handle")?.into();
        let id = string(args, "id")?.into();
        let properties = data::from_json(args.get("properties").unwrap_or(&Value::Null));
        Ok(Request::Command(Box::new(style::SetProperties { handle, id, properties })))
    }
}
