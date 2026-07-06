# Swift Coding Conventions

*Last updated: November 16, 2025*

This document establishes comprehensive coding standards and style guidelines for Swift packages. These conventions ensure consistency, maintainability, and adherence to Swift best practices across the entire codebase.

---

## Table of Contents

1. [File Organization](#file-organization)
2. [Naming Conventions](#naming-conventions)
3. [Code Structure](#code-structure)
4. [Access Control](#access-control)
5. [Type Declarations](#type-declarations)
6. [Property Declarations](#property-declarations)
7. [Function Declarations](#function-declarations)
8. [Control Flow](#control-flow)
9. [Error Handling](#error-handling)
10. [Concurrency & Async/Await](#concurrency--asyncawait)
11. [Protocols & Extensions](#protocols--extensions)
12. [Generics](#generics)
13. [Comments & Documentation](#comments--documentation)
14. [Formatting & Whitespace](#formatting--whitespace)
15. [Swift-Specific Patterns](#swift-specific-patterns)
16. [Package-Specific Conventions](#package-specific-conventions)

---

## File Organization

### Import Statements

```swift
// CORRECT: Organize imports alphabetically, Foundation first if needed
import Foundation
import CoreLocation
import MapKit
import WeatherKit

// INCORRECT: Random order
import WeatherKit
import Foundation
import CoreLocation
```

### File Structure Order

1. Import statements
2. Type declarations (class, struct, enum, protocol)
3. Properties (in order: static, instance)
4. Initializers
5. Lifecycle methods
6. Public methods
7. Internal methods
8. Private methods
9. Nested types (if applicable)

### Single Responsibility

- **One primary type per file** (exceptions for small, tightly-coupled helper types)
- File name must match the primary type name: `FooManager.swift` contains `FooManager` class
- Place closely related types in the same file only when they form a cohesive unit

---

## Naming Conventions

### General Rules

- Use clear, descriptive names that convey intent
- Prefer full words over abbreviations
- Use American English spelling

### Types (Classes, Structs, Enums, Protocols)

```swift
// CORRECT: PascalCase for types
public class FooManager { }
public struct Location { }
public enum FooQuality { }
public protocol FooController { }

// INCORRECT
public class processManager { }  // Wrong case
public struct location { }       // Wrong case
```

### Properties & Variables

```swift
// CORRECT: camelCase for properties and variables
let locationManager = LocationManager()
var subscriptions: [FooSubscription] = []
private let updateInterval: TimeInterval = 60

// INCORRECT
let LocationManager = LocationManager()  // Wrong case
var Subscriptions: [FooSubscription] = []  // Wrong case
```

### Functions & Methods

```swift
// CORRECT: camelCase, descriptive action verbs
func refreshData(for location: Location) async throws -> FooSensor?
func updateLocation(location: Location) -> Void
private func significantLocationChange(previous: Location?, current: Location) -> Bool

// INCORRECT
func RefreshData() { }  // Wrong case
func upd() { }  // Too abbreviated
func location_update() { }  // Snake case
```

### Constants

```swift
// CORRECT: Use static let for type-level constants
public class LocationManager {
    public static let exampleLocation = Location(latitude: 37.0, longitude: -122.0)
}

// CORRECT: camelCase for constant properties
private let updateInterval: TimeInterval = 60
```

### Enums

```swift
// CORRECT: PascalCase for enum name, camelCase for cases
public enum FooQuality {
    case good
    case uncertain
    case bad
    case unknown
}

// CORRECT: Associated value enums
public enum FooSelector: Hashable {
    case channel(Channel)
    case forecast(Forecast)
    case sample(SampleCategory)
}
```

### Protocols

```swift
// CORRECT: Use descriptive protocol names
public protocol FooController { }
public protocol LocationManagerDelegate: Identifiable where ID == UUID { }

// CORRECT: Protocol names ending in -able, -ible indicate capability
protocol Sendable { }  // Standard library example
```

---

## Code Structure

### Braces

```swift
// CORRECT: Opening brace on same line, closing brace on new line
public class FooManager {
    func updateSubscriptions() {
        for subscription in subscriptions {
            subscription.update(timeout: updateInterval)
        }
    }
}

// INCORRECT
public class FooManager
{  // Opening brace on new line
    func updateSubscriptions()
    {
        for subscription in subscriptions {
            subscription.update(timeout: updateInterval) }  // Closing brace on same line
    }
}
```

### Indentation

- Use **4 spaces** for indentation (no tabs)
- Align continuation lines with the opening delimiter

```swift
// CORRECT: 4-space indentation
public init(
    name: String, location: Location, placemark: String?, customData: [String: Any]?,
    measurements: [FooSelector: [FooValue<Dimension>]], timestamp: Date?
) {
    self.name = name
    self.location = location
    self.placemark = placemark
    self.customData = customData
    self.measurements = measurements
    self.timestamp = timestamp
}
```

### Line Length

- Target maximum: **120 characters** per line
- Break long lines at logical points (parameters, operators, closures)

```swift
// CORRECT: Break long function signatures
public func dataWithRetry(
    from url: URL, retryCount: Int = 3, retryInterval: TimeInterval = 1.0,
    delegate: (any URLSessionTaskDelegate)? = nil
) async throws -> (Data, URLResponse) {
    // Implementation
}
```

---

## Access Control

### Access Levels (Most to Least Restrictive)

1. `private` - Only visible within the current declaration
2. `fileprivate` - Visible within the same source file
3. `internal` - Visible within the module (default)
4. `public` - Visible to consumers of the module
5. `open` - Visible and subclassable outside the module

### Package Guidelines

```swift
// CORRECT: Explicit public for exported API
public class FooManager: Identifiable, LocationManagerDelegate {
    public let id = UUID()
    public static let shared = FooManager()

    private let locationManager = LocationManager()  // Internal implementation
    private var location: Location?  // Private state

    public func refreshSubscriptions() {  // Public API
        // Implementation
    }

    private func updateSubscriptions() {  // Private helper
        // Implementation
    }
}
```

### Rules

- **Always explicit**: Mark APIs as `public` explicitly; avoid relying on default `internal`
- **Minimize exposure**: Only expose what consumers need
- **Private by default**: Start with `private`, increase visibility as needed
- **No `open` classes**: Package doesn't require subclassing from consumers

---

## Type Declarations

### Classes

```swift
// CORRECT: Class with protocol conformance
public class FooManager: Identifiable, LocationManagerDelegate {
    public let id = UUID()
    public static let shared = FooManager()

    private init() {
        // Singleton pattern
    }
}

// CORRECT: Subclass with inheritance
public class BarController: FooController {
    public func refreshData(for location: Location) async throws -> FooSensor? {
        // Implementation
    }
}
```

### Structs

```swift
// CORRECT: Simple value type
public struct Location: Equatable, Hashable {
    public let latitude: Double
    public let longitude: Double

    public init(latitude: Double, longitude: Double) {
        self.latitude = latitude
        self.longitude = longitude
    }
}

// CORRECT: Generic struct with computed properties
public struct FooValue<T: Dimension>: Identifiable {
    public let id = UUID()
    public let value: Measurement<T>
    public let quality: FooQuality
    public let timestamp: Date
}
```

### Enums

```swift
// CORRECT: Simple enum
public enum FooQuality {
    case good
    case uncertain
    case bad
    case unknown
}

// CORRECT: Enum with raw values
public enum Channel: Int, CaseIterable {
    case temperature = 0
    case apparentTemperature = 1
    case dewPoint = 2
}

// CORRECT: Enum with associated values
public enum FooSelector: Hashable {
    case channel(Channel)
    case forecast(Forecast)
    case sample(SampleCategory)
}
```

### Protocols

```swift
// CORRECT: Protocol with associated type constraints
public protocol LocationManagerDelegate: Identifiable where ID == UUID {
    func locationManager(didUpdateLocation location: Location) -> Void
}

// CORRECT: Simple protocol
public protocol FooController {
    func refreshData(for location: Location) async throws -> FooSensor?
}
```

---

## Property Declarations

### Stored Properties

```swift
// CORRECT: Property declarations with explicit types
public class FooManager {
    public let id = UUID()  // Type inferred from initializer
    private let locationManager = LocationManager()
    private var location: Location?  // Optional type explicit
    private let updateInterval: TimeInterval = 60  // Explicit type
    private var subscriptions: [FooSubscription] = []  // Explicit initialization
}
```

### Computed Properties

```swift
// CORRECT: Computed property
public struct Location {
    public let latitude: Double
    public let longitude: Double

    public var coordinate: CLLocationCoordinate2D {
        return CLLocationCoordinate2D(latitude: self.latitude, longitude: self.longitude)
    }
}

// CORRECT: Read-only computed property (implicit get)
var isReady: Bool {
    return location != nil && subscriptions.isEmpty == false
}
```

### Property Observers

```swift
// CORRECT: willSet and didSet
var location: Location? {
    willSet {
        print("About to set location to \(newValue)")
    }
    didSet {
        if location != oldValue {
            refreshSubscriptions()
        }
    }
}
```

### Lazy Properties

```swift
// CORRECT: Lazy initialization for expensive resources
lazy var dateFormatter: DateFormatter = {
    let formatter = DateFormatter()
    formatter.dateFormat = "yyyy-MM-dd HH:mm:ss.SSS"
    return formatter
}()
```

---

## Function Declarations

### Basic Structure

```swift
// CORRECT: Function signature formatting
public func refreshData(for location: Location) async throws -> FooSensor? {
    var measurements: [FooSelector: [FooValue<Dimension>]] = [:]
    // Implementation
    return FooSensor(name: "", location: location, measurements: measurements, timestamp: Date.now)
}
```

### Parameter Labels

```swift
// CORRECT: Descriptive external labels
func updateLocation(location: Location) -> Void { }
func add(subscriber: any FooSubscriber, timeout: TimeInterval) { }

// CORRECT: Omit external label with underscore when appropriate
func process(_ data: Data) -> Result { }

// INCORRECT: Redundant labels
func updateLocation(location location: Location) -> Void { }  // Redundant
```

### Default Parameters

```swift
// CORRECT: Default parameters at end
public func dataWithRetry(
    from url: URL,
    retryCount: Int = 3,
    retryInterval: TimeInterval = 1.0,
    delegate: (any URLSessionTaskDelegate)? = nil
) async throws -> (Data, URLResponse) {
    // Implementation
}
```

### Multiple Initializers

```swift
// CORRECT: Convenience initializers calling designated initializers
public struct FooValue<T: Dimension> {
    // Designated initializer (most comprehensive)
    public init(value: Measurement<T>, customData: [String: Any]?, quality: FooQuality, timestamp: Date) {
        self.value = value
        self.customData = customData
        self.quality = quality
        self.timestamp = timestamp
    }

    // Convenience initializers
    public init(value: Measurement<T>, quality: FooQuality, timestamp: Date) {
        self.init(value: value, customData: nil, quality: quality, timestamp: timestamp)
    }

    public init(value: Measurement<T>, quality: FooQuality) {
        self.init(value: value, quality: quality, timestamp: Date.now)
    }

    public init(value: Measurement<T>) {
        self.init(value: value, quality: .unknown)
    }
}
```

### Return Type Void

```swift
// CORRECT: Explicit Void return type
public func updateLocation(location: Location) -> Void {
    // Implementation
}

// ALSO CORRECT: Omit return type for Void
public func updateLocation(location: Location) {
    // Implementation
}
```

---

## Control Flow

### If Statements

```swift
// CORRECT: Standard if statement
if location != nil {
    refreshSubscriptions()
}

// CORRECT: If-let for optional binding
if let location = self.location {
    delegate.locationManager(didUpdateLocation: location)
}

// CORRECT: Guard for early return
guard let location = self.location else {
    return
}

// CORRECT: Multiple conditions
if needsUpdate == true {
    self.location = location
    if let delegate = self.delegate {
        delegate.locationManager(didUpdateLocation: location)
    }
}
```

### Guard Statements

```swift
// CORRECT: Guard for preconditions and early exits
guard NetworkMonitor.shared.isConnected else {
    throw URLError(.notConnectedToInternet)
}

guard let url = URL(string: "https://api.example.com/data") else {
    return nil
}

// CORRECT: Multiple guard conditions
guard let data = data,
      let response = response as? HTTPURLResponse,
      (200...299).contains(response.statusCode) else {
    throw NetworkError.invalidResponse
}
```

### For Loops

```swift
// CORRECT: For-in loops
for subscription in subscriptions {
    subscription.update(timeout: updateInterval)
}

// CORRECT: Enumeration with index
for (index, item) in items.enumerated() {
    print("\(index): \(item)")
}

// CORRECT: Filtering in loop
for subscription in subscriptions where subscription.isPending() {
    subscription.reset()
}
```

### Switch Statements

```swift
// CORRECT: Exhaustive switch on enum
switch quality {
    case .good:
        return "✓"
    case .uncertain:
        return "~"
    case .bad:
        return "✗"
    case .unknown:
        return "?"
}

// CORRECT: Switch with multiple cases
switch connectionType {
    case .wifi, .ethernet:
        return true
    case .cellular:
        return false
    case .unknown:
        return false
}
```

### Ternary Operator

```swift
// CORRECT: Simple conditions
let result = condition ? trueValue : falseValue

// AVOID: Nested ternary (use if-else instead)
let result = condition1 ? value1 : (condition2 ? value2 : value3)  // Hard to read
```

---

## Error Handling

### Error Definitions

```swift
// CORRECT: Custom error enum
enum NetworkError: Error {
    case invalidResponse
    case serverError(statusCode: Int)
    case noData
}
```

### Throwing Functions

```swift
// CORRECT: Function that can throw
public func refreshData(for location: Location) async throws -> FooSensor? {
    let payload = try await RemoteDataService.shared.fetch(for: clLocation)
    // Process remote payload
    return sensor
}
```

### Try-Catch Blocks

```swift
// CORRECT: Standard try-catch
do {
    let (data, response) = try await self.data(from: url, delegate: delegate)
    return (data, response)
} catch {
    lastError = error
    if attempt < retryCount - 1 {
        try await Task.sleep(nanoseconds: UInt64(retryInterval * 1_000_000_000))
        continue
    }
}

// CORRECT: Specific error catching
do {
    let result = try riskyOperation()
    return result
} catch NetworkError.invalidResponse {
    print("Invalid response")
    return nil
} catch {
    print("Unknown error: \(error)")
    return nil
}
```

### Optional Try

```swift
// CORRECT: try? for optional result
if let placemark = try? await geocoder.reverseGeocodeLocation(location).first {
    // Use placemark
}

// CORRECT: try! only when failure is impossible
let config = try! Configuration.load()  // Only if guaranteed to succeed
```

---

## Concurrency & Async/Await

### Async Functions

```swift
// CORRECT: Async function declaration
public func refreshData(for location: Location) async throws -> FooSensor? {
    let payload = try await RemoteDataService.shared.fetch(for: clLocation)
    let placemark = await LocationManager.reverseGeocodeLocation(location: location)
    return FooSensor(/* ... */)
}
```

### Task Creation

```swift
// CORRECT: Create task for async work
Task {
    await delegate.refreshData(location: location)
}

// CORRECT: Task with error handling
Task {
    do {
        let result = try await fetchData()
        process(result)
    } catch {
        print("Error: \(error)")
    }
}
```

### Actor Usage

```swift
// CORRECT: Actor for thread-safe state management
actor NetworkManager {
    private var isConnected = true

    func updateConnectionStatus(_ status: Bool) {
        self.isConnected = status
    }

    func checkConnection() -> Bool {
        return isConnected
    }
}
```

### Sendable Conformance

```swift
// CORRECT: @unchecked Sendable for custom Dimension types
public class UnitExample: Dimension, @unchecked Sendable {
    public static let baseUnits = UnitExample(
        symbol: "units",
        converter: UnitConverterLinear(coefficient: 1.0)
    )
}
```

### Async Sequences

```swift
// CORRECT: Iterating async sequence
for await value in asyncSequence {
    process(value)
}
```

---

## Protocols & Extensions

### Protocol Declarations

```swift
// CORRECT: Protocol with requirements
public protocol FooController {
    func refreshData(for location: Location) async throws -> FooSensor?
}

// CORRECT: Protocol with associated type constraints
public protocol LocationManagerDelegate: Identifiable where ID == UUID {
    func locationManager(didUpdateLocation location: Location) -> Void
}
```

### Protocol Conformance

```swift
// CORRECT: Conformance in type definition
public class FooManager: Identifiable, LocationManagerDelegate {
    // Implementation
}

// CORRECT: Conformance in extension (when appropriate)
extension FooManager: CustomStringConvertible {
    public var description: String {
        return "FooManager with \(subscriptions.count) subscriptions"
    }
}
```

### Extensions

```swift
// CORRECT: Extension to add functionality
extension URLSession {
    public func dataWithRetry(
        from url: URL, retryCount: Int = 3, retryInterval: TimeInterval = 1.0
    ) async throws -> (Data, URLResponse) {
        // Implementation
    }
}

// CORRECT: Extension for protocol conformance
extension Location: Equatable, Hashable {
    // Compiler synthesizes conformance for structs with Equatable/Hashable properties
}
```

### Extension Organization

```swift
// CORRECT: Organize extensions by purpose
// File: FooManager.swift

public class FooManager {
    // Core implementation
}

// MARK: - LocationManagerDelegate
extension FooManager: LocationManagerDelegate {
    public func locationManager(didUpdateLocation location: Location) {
        // Implementation
    }
}

// MARK: - Subscription Management
extension FooManager {
    public func add(subscriber: any FooSubscriber, timeout: TimeInterval) {
        // Implementation
    }
}
```

---

## Generics

### Generic Types

```swift
// CORRECT: Generic struct with type constraints
public struct FooValue<T: Dimension>: Identifiable {
    public let id = UUID()
    public let value: Measurement<T>
    public let quality: FooQuality
}
```

### Generic Functions

```swift
// CORRECT: Generic function with constraints
func measure<T: Dimension>(_ value: Double, unit: T) -> Measurement<T> {
    return Measurement(value: value, unit: unit)
}
```

### Associated Types

```swift
// CORRECT: Protocol with associated type
protocol Container {
    associatedtype Item
    var items: [Item] { get set }
    mutating func add(_ item: Item)
}
```

### Type Erasure

```swift
// CORRECT: Using 'any' for existential types
private var subscribers: [UUID: any FooSubscriber] = [:]

public func add(subscriber: any FooSubscriber, timeout: TimeInterval) {
    subscribers[subscriber.id] = subscriber
}
```

---

## Comments & Documentation

### Single-Line Comments

```swift
// CORRECT: Comment explains why, not what
// Check if device is connected before attempting network request
guard NetworkMonitor.shared.isConnected else {
    throw URLError(.notConnectedToInternet)
}

// INCORRECT: States the obvious
// Set location to new location
self.location = location
```

### Multi-Line Comments

```swift
// CORRECT: Use single-line style for multi-line comments
// This function performs exponential backoff retry logic
// for network requests. It checks connectivity before each
// attempt and throws immediately if connection is lost.
```

### Documentation Comments

```swift
// CORRECT: DocC-style documentation
/// A simple and fast logging facility with support for different log levels and detailed timestamps.
public class Logger {
    /// Represents different log levels
    public enum Level: String {
        case debug = "DEBUG"
        case info = "INFO"
    }

    /// Creates a new Logger instance
    /// - Parameters:
    ///   - minimumLevel: Minimum level of logs to display
    ///   - showColors: Whether to use ANSI colors in console output
    ///   - dateFormat: Format string for timestamps (default: "yyyy-MM-dd HH:mm:ss.SSS")
    ///   - logFile: Path to file for writing logs (optional)
    public init(
        minimumLevel: Level = .debug,
        showColors: Bool = true,
        dateFormat: String = "yyyy-MM-dd HH:mm:ss.SSS",
        logFile: String? = nil
    ) {
        // Implementation
    }
}
```

### MARK Comments

```swift
// CORRECT: Use MARK to organize code sections
public class BarController {
    // MARK: - Properties
    private let service = RemoteDataService.shared

    // MARK: - Initialization
    public init() { }

    // MARK: - Public Methods
    public func refreshData(for location: Location) async throws -> FooSensor? {
        // Implementation
    }

    // MARK: - Private Helpers
    private func processRemotePayload(_ data: RemotePayload) -> FooSensor {
        // Implementation
    }
}
```

### TODO/FIXME Comments

```swift
// TODO: Implement caching mechanism for remote data
// FIXME: Handle edge case when location is exactly on boundary
// NOTE: This assumes the API always returns valid data
```

---

## Formatting & Whitespace

### Blank Lines

```swift
// CORRECT: Blank line between logical sections
public class FooManager {
    public let id = UUID()
    public static let shared = FooManager()

    private let locationManager = LocationManager()
    private var location: Location?

    private init() {
        self.locationManager.delegate = self
    }

    public func refreshSubscriptions() {
        // Implementation
    }
}
```

### Spacing

```swift
// CORRECT: Space after comma, around operators
let values = [1, 2, 3, 4]
let sum = a + b
let range = 0.0 ... 100.0

// CORRECT: No space around range operators
for i in 0..<count { }
let range = 0...10

// CORRECT: No space before colon, space after
var measurements: [FooSelector: [FooValue<Dimension>]] = [:]
func add(subscriber: any FooSubscriber, timeout: TimeInterval) { }

// INCORRECT
let values=[1,2,3,4]  // Missing spaces
let sum=a+b  // Missing spaces
var dict : [String : Int]  // Spaces before colons
```

### Trailing Whitespace

```swift
// AVOID: Trailing whitespace at end of lines
func process() {
    let value = 10___
}  // Remove trailing spaces

// CORRECT: No trailing whitespace
func process() {
    let value = 10
}
```

### Empty Lines at File End

```swift
// CORRECT: Single empty line at end of file
public class FooManager {
    // Implementation
}

// ← One blank line here, then EOF
```

---

## Swift-Specific Patterns

### Optionals

```swift
// CORRECT: Optional binding with if-let
if let location = self.location {
    process(location)
}

// CORRECT: Optional binding with guard
guard let location = self.location else {
    return
}

// CORRECT: Optional chaining
let count = subscribers[id]?.subscriptions.count

// CORRECT: Nil coalescing
let value = optionalValue ?? defaultValue

// AVOID: Force unwrapping (use only when absolutely certain)
let value = optionalValue!  // Only if guaranteed non-nil
```

### Type Inference

```swift
// CORRECT: Let Swift infer obvious types
let manager = FooManager.shared
let id = UUID()
let values = [1, 2, 3]

// CORRECT: Explicit types for clarity
let timeout: TimeInterval = 60
let measurements: [FooSelector: [FooValue<Dimension>]] = [:]

// AVOID: Redundant type annotations
let manager: FooManager = FooManager.shared  // Type obvious
```

### Closures

```swift
// CORRECT: Trailing closure syntax
Timer.scheduledTimer(withTimeInterval: updateInterval, repeats: true) { _ in
    self.updateSubscriptions()
}

// CORRECT: Explicit closure parameters
items.map { item in
    return item.value * 2
}

// CORRECT: Shorthand when simple
items.map { $0.value * 2 }

// CORRECT: Multiple trailing closures (Swift 5.3+)
UIView.animate(withDuration: 0.3) {
    view.alpha = 0
} completion: { _ in
    view.removeFromSuperview()
}
```

### Collections

```swift
// CORRECT: Array initialization
var subscriptions: [FooSubscription] = []
let values = [1, 2, 3, 4, 5]

// CORRECT: Dictionary initialization
var measurements: [FooSelector: [FooValue<Dimension>]] = [:]
let dict = ["key": "value"]

// CORRECT: Set initialization
let uniqueIds: Set<UUID> = []
```

### Lazy Evaluation

```swift
// CORRECT: Lazy sequences for performance
let largeArray = (0..<1_000_000)
let evenNumbers = largeArray.lazy.filter { $0 % 2 == 0 }
```

### Property Wrappers

```swift
// CORRECT: Custom property wrapper usage
@Published var measurements: [FooValue<Dimension>] = []

// CORRECT: UserDefaults property wrapper
@AppStorage("refreshInterval") var refreshInterval: TimeInterval = 60
```

---

## Package-Specific Conventions

### Public API Patterns

```swift
// CORRECT: Controller pattern
public class BarController: FooController {
    public func refreshData(for location: Location) async throws -> FooSensor? {
        // Fetch data from service
        // Process into FooSensor
        // Return structured data
    }
}

// CORRECT: Service pattern (stateless)
public class FooService {
    static func fetchRegions(for location: Location, radius: Double) async throws -> Data? {
        // Perform HTTP request
        // Return raw data
    }
}

// CORRECT: Transformer pattern
public class BarTransformer: DataTransformer {
    override public func renderCurrent(measurements: [FooSelector: [FooValue<Dimension>]])
        -> [FooSelector: FooValue<Dimension>] {
        // Transform raw measurements into current values
    }
}
```

### Data Flow Pattern

```swift
// Service (HTTP) → Controller (Parse) → Transformer (Process) → Consumer (Display)

// 1. Service: Fetch raw data
let data = try await FooService.fetchSample(id: regionId)

// 2. Controller: Parse and structure
let sensor = try await controller.refreshData(for: location)

// 3. Transformer: Process for display
let transformer = BarTransformer()
try transformer.renderData(sensor: sensor)

// 4. Consumer uses: transformer.current, transformer.displaySnapshot, etc.
```

### Process Architecture

```swift
// CORRECT: FooValue with quality assessment
let temperature = Measurement<Dimension>(value: 20.5, unit: UnitTemperature.celsius)
let processValue = FooValue(value: temperature, quality: .good, timestamp: Date.now)

// CORRECT: FooSensor with measurements
let sensor = FooSensor(
    name: "Example Station",
    location: location,
    placemark: "Example City, Region",
    customData: ["icon": "cloud.sun"],
    measurements: measurements,
    timestamp: Date.now
)

// CORRECT: FooSelector for data organization
measurements[.channel(.temperature)] = [processValue]
measurements[.channel(.humidity)] = [humidityValue]
```

### Custom Units Pattern

```swift
// CORRECT: Custom Dimension subclass with @unchecked Sendable
public class UnitExample: Dimension, @unchecked Sendable {
    public static let baseUnits = UnitExample(
        symbol: "units",
        converter: UnitConverterLinear(coefficient: 1.0)
    )

    public static let microUnits = UnitExample(
        symbol: "µunits",
        converter: UnitConverterLinear(coefficient: 0.000001)
    )

    override public class func baseUnit() -> Self {
        return baseUnits as! Self
    }
}
```

### Subscription Pattern

```swift
// CORRECT: FooManager subscription system
public func add(subscriber: any FooSubscriber, timeout: TimeInterval) {
    subscriptions.append(FooSubscription(id: subscriber.id, timeout: timeout * 60))
    subscribers[subscriber.id] = subscriber
}

// CORRECT: FooSubscriber protocol implementation
public protocol FooSubscriber: Identifiable {
    func refreshData(location: Location) async
    func resetData() async
}
```

### Location-Based Updates

```swift
// CORRECT: LocationManagerDelegate pattern
public protocol LocationManagerDelegate: Identifiable where ID == UUID {
    func locationManager(didUpdateLocation location: Location) -> Void
}

// CORRECT: Significant location change detection
private func significantLocationChange(previous: Location?, current: Location) -> Bool {
    guard let previous = previous else { return true }
    let deadband = Measurement(value: 100.0, unit: UnitLength.meters)
    let distance = haversineDistance(location_0: previous, location_1: current)
    return distance > deadband
}
```

### Network Resilience Pattern

```swift
// CORRECT: URLSession extension with retry logic
extension URLSession {
    public func dataWithRetry(
        from url: URL, retryCount: Int = 3, retryInterval: TimeInterval = 1.0
    ) async throws -> (Data, URLResponse) {
        var lastError: Error?

        guard NetworkMonitor.shared.isConnected else {
            throw URLError(.notConnectedToInternet)
        }

        for attempt in 0..<retryCount {
            do {
                let (data, response) = try await self.data(from: url)
                return (data, response)
            } catch {
                lastError = error
                if attempt < retryCount - 1 {
                    try await Task.sleep(nanoseconds: UInt64(retryInterval * 1_000_000_000))
                }
            }
        }
        throw lastError ?? URLError(.unknown)
    }
}
```

### Logging Pattern

```swift
// CORRECT: Use Logger utility for structured logging
logger.debug("Fetching sample measurement regions...")
let data = try await service.fetch()
logger.debug("Fetched sample measurement regions.")

logger.error("Failed to parse response: \(error)")
```

### Platform Independence

```swift
// CORRECT: Platform conditionals for OS-specific code
#if os(iOS)
locationManager.allowsBackgroundLocationUpdates = true
locationManager.pausesLocationUpdatesAutomatically = false
#else
locationManager.desiredAccuracy = kCLLocationAccuracyKilometer
#endif

// AVOID: UI framework dependencies (SwiftUI, UIKit, AppKit) in package
// Keep package focused on business logic and data processing
```

---

## Summary Checklist

### Before Committing Code

- [ ] All public APIs have explicit `public` access control
- [ ] All types, functions, and properties follow naming conventions
- [ ] Code is formatted with 4-space indentation
- [ ] No trailing whitespace
- [ ] Documentation comments for public APIs
- [ ] Error handling is comprehensive
- [ ] Async/await used consistently throughout
- [ ] No platform-specific UI dependencies (SwiftUI, UIKit, AppKit)
- [ ] Custom `Dimension` types conform to `@unchecked Sendable`
- [ ] Protocol conformance is clear and explicit
- [ ] MARK comments organize code sections
- [ ] No force unwrapping (!) unless absolutely safe
- [ ] Follows established package patterns (Controller/Service/Transformer)

### Code Review Focus Areas

1. **Access Control**: Correct use of public/private/internal
2. **Naming**: Clear, descriptive, follows conventions
3. **Error Handling**: Comprehensive try-catch, meaningful errors
4. **Concurrency**: Proper async/await, actor usage, Sendable conformance
5. **Architecture**: Follows Controller/Service/Transformer pattern
6. **Documentation**: Public APIs documented, complex logic explained
7. **Platform Independence**: No UI framework dependencies
8. **Performance**: Efficient algorithms, lazy evaluation where appropriate
9. **Safety**: No force unwrapping, proper optional handling
10. **Consistency**: Matches existing codebase patterns

---

*This document is maintained alongside AGENTS.md and should be updated when new patterns emerge or conventions change.*
