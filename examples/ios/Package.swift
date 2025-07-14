// swift-tools-version: 5.7
import PackageDescription

let package = Package(
    name: "NoiseMobileExample",
    platforms: [
        .iOS(.v15),
        .macOS(.v12)
    ],
    products: [
        .library(
            name: "NoiseMobileSwift",
            targets: ["NoiseMobileSwift"]),
    ],
    targets: [
        .target(
            name: "NoiseMobileSwift",
            dependencies: ["NoiseMobileFFI"],
            path: "Sources",
            sources: ["NoiseSession.swift", "BLENoiseTransport.swift"]
        ),
        .binaryTarget(
            name: "NoiseMobileFFI",
            path: "../../target/NoiseMobile.xcframework"
        ),
        .executableTarget(
            name: "NoiseMobileExample",
            dependencies: ["NoiseMobileSwift"],
            path: "Sources",
            sources: ["ExampleApp.swift"]
        )
    ]
)