//! The format's enums grafta uses, with every value the schemas of Figma's
//! files list. Written by `tests/schemas.rs` from the fixtures; do not edit.

crate::wire! {
    pub enum AnimationPhase {
        In = "IN",
        Out = "OUT",
    }
}

crate::wire! {
    pub enum AnimationType {
        Fade = "FADE",
        None = "NONE",
        SlideFromBottom = "SLIDE_FROM_BOTTOM",
        SlideFromLeft = "SLIDE_FROM_LEFT",
        SlideFromRight = "SLIDE_FROM_RIGHT",
        SlideFromTop = "SLIDE_FROM_TOP",
    }
}

crate::wire! {
    pub enum BlendMode {
        Color = "COLOR",
        ColorBurn = "COLOR_BURN",
        ColorDodge = "COLOR_DODGE",
        Darken = "DARKEN",
        Difference = "DIFFERENCE",
        Exclusion = "EXCLUSION",
        HardLight = "HARD_LIGHT",
        Hue = "HUE",
        Lighten = "LIGHTEN",
        LinearBurn = "LINEAR_BURN",
        LinearDodge = "LINEAR_DODGE",
        Luminosity = "LUMINOSITY",
        Multiply = "MULTIPLY",
        Normal = "NORMAL",
        Overlay = "OVERLAY",
        PassThrough = "PASS_THROUGH",
        Saturation = "SATURATION",
        Screen = "SCREEN",
        SoftLight = "SOFT_LIGHT",
        Unset = "UNSET",
    }
}

crate::wire! {
    pub enum ComponentPropNodeField {
        InheritFillStyleId = "INHERIT_FILL_STYLE_ID",
        OverriddenSymbolId = "OVERRIDDEN_SYMBOL_ID",
        SlotContentId = "SLOT_CONTENT_ID",
        TextData = "TEXT_DATA",
        Visible = "VISIBLE",
    }
}

crate::wire! {
    pub enum ComponentPropType {
        Bool = "BOOL",
        Circle = "CIRCLE",
        CirclePoint = "CIRCLE_POINT",
        Color = "COLOR",
        ColorArray = "COLOR_ARRAY",
        ColorPoint = "COLOR_POINT",
        Easing = "EASING",
        Gradient = "GRADIENT",
        Image = "IMAGE",
        InstanceSwap = "INSTANCE_SWAP",
        Line = "LINE",
        Node = "NODE",
        Number = "NUMBER",
        Paint = "PAINT",
        Rotation3d = "ROTATION_3D",
        Slot = "SLOT",
        Text = "TEXT",
        Variant = "VARIANT",
        Vector = "VECTOR",
    }
}

crate::wire! {
    pub enum ConnectionType {
        Back = "BACK",
        Close = "CLOSE",
        Conditional = "CONDITIONAL",
        InternalNode = "INTERNAL_NODE",
        None = "NONE",
        ObjectAnimation = "OBJECT_ANIMATION",
        SetVariable = "SET_VARIABLE",
        SetVariableMode = "SET_VARIABLE_MODE",
        UpdateAnimationTimelineState = "UPDATE_ANIMATION_TIMELINE_STATE",
        UpdateMediaRuntime = "UPDATE_MEDIA_RUNTIME",
        Url = "URL",
    }
}

crate::wire! {
    pub enum ConnectorLineStyle {
        Curved = "CURVED",
        Elbowed = "ELBOWED",
        Straight = "STRAIGHT",
    }
}

crate::wire! {
    pub enum ConnectorMagnet {
        Absolute = "ABSOLUTE",
        Auto = "AUTO",
        AutoHorizontal = "AUTO_HORIZONTAL",
        Bottom = "BOTTOM",
        Center = "CENTER",
        Edge = "EDGE",
        Left = "LEFT",
        None = "NONE",
        Right = "RIGHT",
        Top = "TOP",
    }
}

crate::wire! {
    pub enum ConstraintType {
        Center = "CENTER",
        FixedMax = "FIXED_MAX",
        FixedMin = "FIXED_MIN",
        Max = "MAX",
        Min = "MIN",
        Scale = "SCALE",
        Stretch = "STRETCH",
    }
}

crate::wire! {
    pub enum EasingType {
        CustomCubic = "CUSTOM_CUBIC",
        CustomSpring = "CUSTOM_SPRING",
        EaseIn = "EASE_IN",
        GentleSpring = "GENTLE_SPRING",
        Hold = "HOLD",
        InoutBackCubic = "INOUT_BACK_CUBIC",
        InoutCubic = "INOUT_CUBIC",
        InBackCubic = "IN_BACK_CUBIC",
        InCubic = "IN_CUBIC",
        Linear = "LINEAR",
        OutBackCubic = "OUT_BACK_CUBIC",
        OutCubic = "OUT_CUBIC",
        Spring = "SPRING",
        SpringPresetOne = "SPRING_PRESET_ONE",
        SpringPresetThree = "SPRING_PRESET_THREE",
        SpringPresetTwo = "SPRING_PRESET_TWO",
    }
}

crate::wire! {
    pub enum EffectType {
        BackgroundBlur = "BACKGROUND_BLUR",
        Custom = "CUSTOM",
        DropShadow = "DROP_SHADOW",
        ForegroundBlur = "FOREGROUND_BLUR",
        Glass = "GLASS",
        Grain = "GRAIN",
        InnerShadow = "INNER_SHADOW",
        Noise = "NOISE",
        Repeat = "REPEAT",
        Symmetry = "SYMMETRY",
    }
}

crate::wire! {
    pub enum ImageScaleMode {
        Fill = "FILL",
        Fit = "FIT",
        Stretch = "STRETCH",
        Tile = "TILE",
    }
}

crate::wire! {
    pub enum InteractionType {
        AfterTimeout = "AFTER_TIMEOUT",
        Drag = "DRAG",
        MouseDown = "MOUSE_DOWN",
        MouseEnter = "MOUSE_ENTER",
        MouseIn = "MOUSE_IN",
        MouseLeave = "MOUSE_LEAVE",
        MouseOut = "MOUSE_OUT",
        MouseUp = "MOUSE_UP",
        None = "NONE",
        OnClick = "ON_CLICK",
        OnHover = "ON_HOVER",
        OnKeyDown = "ON_KEY_DOWN",
        OnMediaEnd = "ON_MEDIA_END",
        OnMediaHit = "ON_MEDIA_HIT",
        OnPress = "ON_PRESS",
        OnVoice = "ON_VOICE",
    }
}

crate::wire! {
    pub enum LineType {
        Blockquote = "BLOCKQUOTE",
        Checklist = "CHECKLIST",
        Header = "HEADER",
        OrderedList = "ORDERED_LIST",
        Plain = "PLAIN",
        UnorderedList = "UNORDERED_LIST",
    }
}

crate::wire! {
    pub enum MaskType {
        Alpha = "ALPHA",
        Luminance = "LUMINANCE",
        Outline = "OUTLINE",
    }
}

crate::wire! {
    pub enum NavigationType {
        Navigate = "NAVIGATE",
        Overlay = "OVERLAY",
        ScrollTo = "SCROLL_TO",
        Swap = "SWAP",
        SwapState = "SWAP_STATE",
    }
}

crate::wire! {
    pub enum NodePhase {
        Created = "CREATED",
        Removed = "REMOVED",
    }
}

crate::wire! {
    pub enum NodeType {
        AnimationPresetInstance = "ANIMATION_PRESET_INSTANCE",
        AssistedLayout = "ASSISTED_LAYOUT",
        BinaryFile = "BINARY_FILE",
        BooleanOperation = "BOOLEAN_OPERATION",
        Brush = "BRUSH",
        Canvas = "CANVAS",
        CmsRichText = "CMS_RICH_TEXT",
        CodeBlock = "CODE_BLOCK",
        CodeComponent = "CODE_COMPONENT",
        CodeEmbed = "CODE_EMBED",
        CodeFile = "CODE_FILE",
        CodeInstance = "CODE_INSTANCE",
        CodeLayer = "CODE_LAYER",
        CodeLibrary = "CODE_LIBRARY",
        Connector = "CONNECTOR",
        CustomEffectInstance = "CUSTOM_EFFECT_INSTANCE",
        Document = "DOCUMENT",
        Ellipse = "ELLIPSE",
        EmbeddedPrototype = "EMBEDDED_PROTOTYPE",
        Frame = "FRAME",
        Group = "GROUP",
        Highlight = "HIGHLIGHT",
        Instance = "INSTANCE",
        InteractiveSlideElement = "INTERACTIVE_SLIDE_ELEMENT",
        Jsx = "JSX",
        Keyframe = "KEYFRAME",
        KeyframeTrack = "KEYFRAME_TRACK",
        Line = "LINE",
        ManagedString = "MANAGED_STRING",
        Media = "MEDIA",
        Module = "MODULE",
        NativeCodeLayerInstance = "NATIVE_CODE_LAYER_INSTANCE",
        None = "NONE",
        ReactFiber = "REACT_FIBER",
        Rectangle = "RECTANGLE",
        RegularPolygon = "REGULAR_POLYGON",
        Repeater = "REPEATER",
        ResponsiveNodeSet = "RESPONSIVE_NODE_SET",
        ResponsiveSet = "RESPONSIVE_SET",
        RoundedRectangle = "ROUNDED_RECTANGLE",
        Section = "SECTION",
        SectionOverlay = "SECTION_OVERLAY",
        ShapeWithText = "SHAPE_WITH_TEXT",
        Slice = "SLICE",
        Slide = "SLIDE",
        SlideGrid = "SLIDE_GRID",
        SlideRow = "SLIDE_ROW",
        SpecBlock = "SPEC_BLOCK",
        Stamp = "STAMP",
        Star = "STAR",
        Sticky = "STICKY",
        Symbol = "SYMBOL",
        Table = "TABLE",
        TableCell = "TABLE_CELL",
        Text = "TEXT",
        TextPath = "TEXT_PATH",
        ToolInstance = "TOOL_INSTANCE",
        Transform = "TRANSFORM",
        Variable = "VARIABLE",
        VariableOverride = "VARIABLE_OVERRIDE",
        VariableSet = "VARIABLE_SET",
        Vector = "VECTOR",
        WashiTape = "WASHI_TAPE",
        Webpage = "WEBPAGE",
        Widget = "WIDGET",
    }
}

crate::wire! {
    pub enum NumberUnits {
        Percent = "PERCENT",
        Pixels = "PIXELS",
        Raw = "RAW",
    }
}

crate::wire! {
    pub enum OverlayBackgroundInteraction {
        CloseOnClickOutside = "CLOSE_ON_CLICK_OUTSIDE",
        None = "NONE",
    }
}

crate::wire! {
    pub enum OverlayBackgroundType {
        None = "NONE",
        SolidColor = "SOLID_COLOR",
    }
}

crate::wire! {
    pub enum OverlayPositionType {
        BottomCenter = "BOTTOM_CENTER",
        BottomLeft = "BOTTOM_LEFT",
        BottomRight = "BOTTOM_RIGHT",
        Center = "CENTER",
        Manual = "MANUAL",
        TopCenter = "TOP_CENTER",
        TopLeft = "TOP_LEFT",
        TopRight = "TOP_RIGHT",
    }
}

crate::wire! {
    pub enum PaintType {
        Custom = "CUSTOM",
        Emoji = "EMOJI",
        GradientAngular = "GRADIENT_ANGULAR",
        GradientDiamond = "GRADIENT_DIAMOND",
        GradientLinear = "GRADIENT_LINEAR",
        GradientRadial = "GRADIENT_RADIAL",
        Image = "IMAGE",
        Noise = "NOISE",
        Pattern = "PATTERN",
        Solid = "SOLID",
        Video = "VIDEO",
    }
}

crate::wire! {
    pub enum PrototypeDeviceType {
        Custom = "CUSTOM",
        None = "NONE",
        Presentation = "PRESENTATION",
        Preset = "PRESET",
    }
}

crate::wire! {
    pub enum ScrollBehavior {
        FixedWhenChildOfScrollingFrame = "FIXED_WHEN_CHILD_OF_SCROLLING_FRAME",
        Scrolls = "SCROLLS",
        StickyScrolls = "STICKY_SCROLLS",
    }
}

crate::wire! {
    pub enum ScrollDirection {
        Both = "BOTH",
        Horizontal = "HORIZONTAL",
        None = "NONE",
        Vertical = "VERTICAL",
    }
}

crate::wire! {
    pub enum ShapeWithTextType {
        ArrowLeft = "ARROW_LEFT",
        ArrowRight = "ARROW_RIGHT",
        Chevron = "CHEVRON",
        Diamond = "DIAMOND",
        DocumentMultiple = "DOCUMENT_MULTIPLE",
        DocumentSingle = "DOCUMENT_SINGLE",
        Ellipse = "ELLIPSE",
        EngDatabase = "ENG_DATABASE",
        EngFile = "ENG_FILE",
        EngFolder = "ENG_FOLDER",
        EngQueue = "ENG_QUEUE",
        Hexagon = "HEXAGON",
        InternalStorage = "INTERNAL_STORAGE",
        ManualInput = "MANUAL_INPUT",
        Octagon = "OCTAGON",
        Or = "OR",
        ParallelogramLeft = "PARALLELOGRAM_LEFT",
        ParallelogramRight = "PARALLELOGRAM_RIGHT",
        Pentagon = "PENTAGON",
        Plus = "PLUS",
        PredefinedProcess = "PREDEFINED_PROCESS",
        RoundedRectangle = "ROUNDED_RECTANGLE",
        Shield = "SHIELD",
        SpeechBubble = "SPEECH_BUBBLE",
        Square = "SQUARE",
        Star = "STAR",
        SummingJunction = "SUMMING_JUNCTION",
        Trapezoid = "TRAPEZOID",
        TriangleDown = "TRIANGLE_DOWN",
        TriangleUp = "TRIANGLE_UP",
    }
}

crate::wire! {
    pub enum SourceDirectionality {
        Auto = "AUTO",
        Ltr = "LTR",
        Rtl = "RTL",
    }
}

crate::wire! {
    pub enum StackAlign {
        Baseline = "BASELINE",
        Center = "CENTER",
        Max = "MAX",
        Min = "MIN",
    }
}

crate::wire! {
    pub enum StackCounterAlign {
        Auto = "AUTO",
        Baseline = "BASELINE",
        Center = "CENTER",
        Max = "MAX",
        Min = "MIN",
        Stretch = "STRETCH",
    }
}

crate::wire! {
    pub enum StackJustify {
        Center = "CENTER",
        Max = "MAX",
        Min = "MIN",
        SpaceAround = "SPACE_AROUND",
        SpaceBetween = "SPACE_BETWEEN",
        SpaceEvenly = "SPACE_EVENLY",
        SpaceEvenlyCss = "SPACE_EVENLY_CSS",
    }
}

crate::wire! {
    pub enum StackMode {
        Grid = "GRID",
        Horizontal = "HORIZONTAL",
        None = "NONE",
        Vertical = "VERTICAL",
    }
}

crate::wire! {
    pub enum StackPositioning {
        Absolute = "ABSOLUTE",
        Auto = "AUTO",
    }
}

crate::wire! {
    pub enum StackSize {
        Fixed = "FIXED",
        ResizeToFit = "RESIZE_TO_FIT",
        ResizeToFitWithImplicitSize = "RESIZE_TO_FIT_WITH_IMPLICIT_SIZE",
    }
}

crate::wire! {
    pub enum StackWrap {
        NoWrap = "NO_WRAP",
        Wrap = "WRAP",
    }
}

crate::wire! {
    pub enum StrokeAlign {
        Center = "CENTER",
        Inside = "INSIDE",
        Offset = "OFFSET",
        Outside = "OUTSIDE",
    }
}

crate::wire! {
    pub enum StrokeCap {
        ArrowEquilateral = "ARROW_EQUILATERAL",
        ArrowLines = "ARROW_LINES",
        CircleFilled = "CIRCLE_FILLED",
        DiamondFilled = "DIAMOND_FILLED",
        ErdExactlyOne = "ERD_EXACTLY_ONE",
        ErdMany = "ERD_MANY",
        ErdOne = "ERD_ONE",
        ErdOneOrMore = "ERD_ONE_OR_MORE",
        ErdZeroOrMore = "ERD_ZERO_OR_MORE",
        ErdZeroOrOne = "ERD_ZERO_OR_ONE",
        Highlight = "HIGHLIGHT",
        None = "NONE",
        Round = "ROUND",
        Square = "SQUARE",
        TriangleFilled = "TRIANGLE_FILLED",
        WashiTape1 = "WASHI_TAPE_1",
        WashiTape2 = "WASHI_TAPE_2",
        WashiTape3 = "WASHI_TAPE_3",
        WashiTape4 = "WASHI_TAPE_4",
        WashiTape5 = "WASHI_TAPE_5",
        WashiTape6 = "WASHI_TAPE_6",
    }
}

crate::wire! {
    pub enum StrokeJoin {
        Bevel = "BEVEL",
        Miter = "MITER",
        Round = "ROUND",
    }
}

crate::wire! {
    pub enum StyleType {
        Animation = "ANIMATION",
        Effect = "EFFECT",
        Export = "EXPORT",
        Fill = "FILL",
        Grid = "GRID",
        None = "NONE",
        Stroke = "STROKE",
        Text = "TEXT",
    }
}

crate::wire! {
    pub enum TextAlignHorizontal {
        Center = "CENTER",
        Justified = "JUSTIFIED",
        Left = "LEFT",
        Right = "RIGHT",
    }
}

crate::wire! {
    pub enum TextAlignVertical {
        Bottom = "BOTTOM",
        Center = "CENTER",
        Top = "TOP",
    }
}

crate::wire! {
    pub enum TextAutoResize {
        Height = "HEIGHT",
        None = "NONE",
        WidthAndHeight = "WIDTH_AND_HEIGHT",
    }
}

crate::wire! {
    pub enum TextCase {
        Lower = "LOWER",
        Original = "ORIGINAL",
        SmallCaps = "SMALL_CAPS",
        SmallCapsForced = "SMALL_CAPS_FORCED",
        Title = "TITLE",
        Upper = "UPPER",
    }
}

crate::wire! {
    pub enum TextDecoration {
        None = "NONE",
        Strikethrough = "STRIKETHROUGH",
        Underline = "UNDERLINE",
    }
}

crate::wire! {
    pub enum TransitionType {
        Dissolve = "DISSOLVE",
        Fade = "FADE",
        InstantTransition = "INSTANT_TRANSITION",
        MagicMove = "MAGIC_MOVE",
        MoveFromBottom = "MOVE_FROM_BOTTOM",
        MoveFromLeft = "MOVE_FROM_LEFT",
        MoveFromRight = "MOVE_FROM_RIGHT",
        MoveFromTop = "MOVE_FROM_TOP",
        MoveOutToBottom = "MOVE_OUT_TO_BOTTOM",
        MoveOutToLeft = "MOVE_OUT_TO_LEFT",
        MoveOutToRight = "MOVE_OUT_TO_RIGHT",
        MoveOutToTop = "MOVE_OUT_TO_TOP",
        PushFromBottom = "PUSH_FROM_BOTTOM",
        PushFromLeft = "PUSH_FROM_LEFT",
        PushFromRight = "PUSH_FROM_RIGHT",
        PushFromTop = "PUSH_FROM_TOP",
        ScrollAnimate = "SCROLL_ANIMATE",
        SlideFromBottom = "SLIDE_FROM_BOTTOM",
        SlideFromLeft = "SLIDE_FROM_LEFT",
        SlideFromRight = "SLIDE_FROM_RIGHT",
        SlideFromTop = "SLIDE_FROM_TOP",
        SlideOutToBottom = "SLIDE_OUT_TO_BOTTOM",
        SlideOutToLeft = "SLIDE_OUT_TO_LEFT",
        SlideOutToRight = "SLIDE_OUT_TO_RIGHT",
        SlideOutToTop = "SLIDE_OUT_TO_TOP",
        SmartAnimate = "SMART_ANIMATE",
    }
}

crate::wire! {
    pub enum TriggerDevice {
        Keyboard = "KEYBOARD",
        Ps4 = "PS4",
        SwitchPro = "SWITCH_PRO",
        UnknownController = "UNKNOWN_CONTROLLER",
        XboxOne = "XBOX_ONE",
    }
}

crate::wire! {
    pub enum VariableDataType {
        Alias = "ALIAS",
        Boolean = "BOOLEAN",
        Circle = "CIRCLE",
        CirclePoint = "CIRCLE_POINT",
        CmsAlias = "CMS_ALIAS",
        Color = "COLOR",
        ColorArray = "COLOR_ARRAY",
        ColorPoint = "COLOR_POINT",
        Date = "DATE",
        Easing = "EASING",
        Expression = "EXPRESSION",
        Float = "FLOAT",
        FontStyle = "FONT_STYLE",
        Gradient = "GRADIENT",
        GradientPaint = "GRADIENT_PAINT",
        Image = "IMAGE",
        Invalid = "INVALID",
        JsRuntimeAlias = "JS_RUNTIME_ALIAS",
        KeyframeTrackId = "KEYFRAME_TRACK_ID",
        KeyframeTrackParameterData = "KEYFRAME_TRACK_PARAMETER_DATA",
        Length = "LENGTH",
        Line = "LINE",
        Link = "LINK",
        ManagedStringAlias = "MANAGED_STRING_ALIAS",
        Map = "MAP",
        NodeFieldAlias = "NODE_FIELD_ALIAS",
        Paint = "PAINT",
        PropRef = "PROP_REF",
        Rotation3d = "ROTATION_3D",
        SlotContentId = "SLOT_CONTENT_ID",
        String = "STRING",
        SymbolId = "SYMBOL_ID",
        TextData = "TEXT_DATA",
        Timing = "TIMING",
        TransitionParameterData = "TRANSITION_PARAMETER_DATA",
        UnitNumber = "UNIT_NUMBER",
        Vector = "VECTOR",
    }
}

crate::wire! {
    pub enum VariableField {
        ArcDataEndingAngle = "ARC_DATA_ENDING_ANGLE",
        ArcDataInnerRadius = "ARC_DATA_INNER_RADIUS",
        ArcDataStartingAngle = "ARC_DATA_STARTING_ANGLE",
        BorderBottomWeight = "BORDER_BOTTOM_WEIGHT",
        BorderLeftWeight = "BORDER_LEFT_WEIGHT",
        BorderRightWeight = "BORDER_RIGHT_WEIGHT",
        BorderTopWeight = "BORDER_TOP_WEIGHT",
        CmsSerializedRichTextData = "CMS_SERIALIZED_RICH_TEXT_DATA",
        CornerRadius = "CORNER_RADIUS",
        DissolveProgress = "DISSOLVE_PROGRESS",
        EasingData = "EASING_DATA",
        FontFamily = "FONT_FAMILY",
        FontSize = "FONT_SIZE",
        FontStyle = "FONT_STYLE",
        FontVariations = "FONT_VARIATIONS",
        GridColumnGap = "GRID_COLUMN_GAP",
        GridRowGap = "GRID_ROW_GAP",
        Height = "HEIGHT",
        Hyperlink = "HYPERLINK",
        KeyframeValue = "KEYFRAME_VALUE",
        LetterSpacing = "LETTER_SPACING",
        LineHeight = "LINE_HEIGHT",
        MaxHeight = "MAX_HEIGHT",
        MaxWidth = "MAX_WIDTH",
        MediaCurrentTime = "MEDIA_CURRENT_TIME",
        MinHeight = "MIN_HEIGHT",
        MinWidth = "MIN_WIDTH",
        Missing = "MISSING",
        MotionRotation = "MOTION_ROTATION",
        MotionScaleX = "MOTION_SCALE_X",
        MotionScaleXy = "MOTION_SCALE_XY",
        MotionScaleY = "MOTION_SCALE_Y",
        MotionShear = "MOTION_SHEAR",
        MotionTransformOriginX = "MOTION_TRANSFORM_ORIGIN_X",
        MotionTransformOriginXy = "MOTION_TRANSFORM_ORIGIN_XY",
        MotionTransformOriginY = "MOTION_TRANSFORM_ORIGIN_Y",
        MotionTranslationX = "MOTION_TRANSLATION_X",
        MotionTranslationXy = "MOTION_TRANSLATION_XY",
        MotionTranslationY = "MOTION_TRANSLATION_Y",
        Opacity = "OPACITY",
        OverriddenSymbolId = "OVERRIDDEN_SYMBOL_ID",
        ParagraphIndent = "PARAGRAPH_INDENT",
        ParagraphSpacing = "PARAGRAPH_SPACING",
        PathTrimEnd = "PATH_TRIM_END",
        PathTrimStart = "PATH_TRIM_START",
        PolygonCount = "POLYGON_COUNT",
        RectangleBottomLeftCornerRadius = "RECTANGLE_BOTTOM_LEFT_CORNER_RADIUS",
        RectangleBottomRightCornerRadius = "RECTANGLE_BOTTOM_RIGHT_CORNER_RADIUS",
        RectangleTopLeftCornerRadius = "RECTANGLE_TOP_LEFT_CORNER_RADIUS",
        RectangleTopRightCornerRadius = "RECTANGLE_TOP_RIGHT_CORNER_RADIUS",
        Rotation = "ROTATION",
        Scene3dPerspective = "SCENE_3D_PERSPECTIVE",
        ScrollOffsetX = "SCROLL_OFFSET_X",
        ScrollOffsetY = "SCROLL_OFFSET_Y",
        SlotContentId = "SLOT_CONTENT_ID",
        StackChildMarginBottom = "STACK_CHILD_MARGIN_BOTTOM",
        StackChildMarginLeft = "STACK_CHILD_MARGIN_LEFT",
        StackChildMarginRight = "STACK_CHILD_MARGIN_RIGHT",
        StackChildMarginTop = "STACK_CHILD_MARGIN_TOP",
        StackCounterSpacing = "STACK_COUNTER_SPACING",
        StackPaddingBottom = "STACK_PADDING_BOTTOM",
        StackPaddingLeft = "STACK_PADDING_LEFT",
        StackPaddingRight = "STACK_PADDING_RIGHT",
        StackPaddingTop = "STACK_PADDING_TOP",
        StackSpacing = "STACK_SPACING",
        StarInnerScale = "STAR_INNER_SCALE",
        StrokeWeight = "STROKE_WEIGHT",
        TextData = "TEXT_DATA",
        TextElementDelay = "TEXT_ELEMENT_DELAY",
        TimelineDuration = "TIMELINE_DURATION",
        Transform3dPerspective = "TRANSFORM_3D_PERSPECTIVE",
        Transform3dRotationX = "TRANSFORM_3D_ROTATION_X",
        Transform3dRotationXyz = "TRANSFORM_3D_ROTATION_XYZ",
        Transform3dRotationY = "TRANSFORM_3D_ROTATION_Y",
        Transform3dRotationZ = "TRANSFORM_3D_ROTATION_Z",
        Transform3dTranslationZ = "TRANSFORM_3D_TRANSLATION_Z",
        VariantProperties = "VARIANT_PROPERTIES",
        Visible = "VISIBLE",
        Width = "WIDTH",
        XPosition = "X_POSITION",
        YPosition = "Y_POSITION",
    }
}

crate::wire! {
    pub enum VariableResolvedDataType {
        Boolean = "BOOLEAN",
        Circle = "CIRCLE",
        CirclePoint = "CIRCLE_POINT",
        Color = "COLOR",
        ColorArray = "COLOR_ARRAY",
        ColorPoint = "COLOR_POINT",
        Easing = "EASING",
        Float = "FLOAT",
        FontStyle = "FONT_STYLE",
        Gradient = "GRADIENT",
        GradientPaint = "GRADIENT_PAINT",
        Image = "IMAGE",
        JsRuntimeAlias = "JS_RUNTIME_ALIAS",
        KeyframeTrackId = "KEYFRAME_TRACK_ID",
        KeyframeTrackParameterData = "KEYFRAME_TRACK_PARAMETER_DATA",
        Length = "LENGTH",
        Line = "LINE",
        Link = "LINK",
        Map = "MAP",
        Paint = "PAINT",
        Rotation3d = "ROTATION_3D",
        SlotContentId = "SLOT_CONTENT_ID",
        String = "STRING",
        SymbolId = "SYMBOL_ID",
        TextData = "TEXT_DATA",
        Timing = "TIMING",
        TransitionParameterData = "TRANSITION_PARAMETER_DATA",
        UnitNumber = "UNIT_NUMBER",
        Vector = "VECTOR",
    }
}

crate::wire! {
    pub enum WindingRule {
        Nonzero = "NONZERO",
        Odd = "ODD",
    }
}

crate::wire::wire_value!(
    AnimationPhase,
    AnimationType,
    BlendMode,
    ComponentPropNodeField,
    ComponentPropType,
    ConnectionType,
    ConnectorLineStyle,
    ConnectorMagnet,
    ConstraintType,
    EasingType,
    EffectType,
    ImageScaleMode,
    InteractionType,
    LineType,
    MaskType,
    NavigationType,
    NodePhase,
    NodeType,
    NumberUnits,
    OverlayBackgroundInteraction,
    OverlayBackgroundType,
    OverlayPositionType,
    PaintType,
    PrototypeDeviceType,
    ScrollBehavior,
    ScrollDirection,
    ShapeWithTextType,
    SourceDirectionality,
    StackAlign,
    StackCounterAlign,
    StackJustify,
    StackMode,
    StackPositioning,
    StackSize,
    StackWrap,
    StrokeAlign,
    StrokeCap,
    StrokeJoin,
    StyleType,
    TextAlignHorizontal,
    TextAlignVertical,
    TextAutoResize,
    TextCase,
    TextDecoration,
    TransitionType,
    TriggerDevice,
    VariableDataType,
    VariableField,
    VariableResolvedDataType,
    WindingRule,
);
