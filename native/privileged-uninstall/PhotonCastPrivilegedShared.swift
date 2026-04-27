import Foundation

let photonCastAppBundleID = "com.photoncast.app"
let helperBundleID = "com.photoncast.privileged-uninstall.helper"
let helperMachServiceName = "com.photoncast.privileged-uninstall.helper"

@objc protocol PhotonCastPrivilegedUninstallProtocol {
    func handle(_ data: Data, withReply reply: @escaping (Data) -> Void)
}

enum PrivilegedCommand: String, Codable {
    case status
    case uninstall
}

enum UninstallMode: String, Codable {
    case trashFirst
    case deleteConfirmed
}

struct PrivilegedRequest: Codable {
    let command: PrivilegedCommand
    let path: String?
    let mode: UninstallMode?
    let requestID: String
}

struct PrivilegedResponse: Codable {
    let ok: Bool
    let requestID: String
    let code: String
    let message: String
    let operation: String?
}

enum ValidationError: Error, CustomStringConvertible {
    case missingPath
    case relativePath(String)
    case cannotCanonicalize(String)
    case notDirectory(String)
    case notAppBundle(String)
    case outsideApplications(String)
    case protectedSystemPath(String)
    case photonCastSelf(String)
    case missingInfoPlist(String)
    case missingBundleIdentifier(String)
    case appleBundle(String)

    var description: String {
        switch self {
        case .missingPath:
            return "missing app path"
        case .relativePath(let path):
            return "path is not absolute: \(path)"
        case .cannotCanonicalize(let path):
            return "cannot canonicalize path: \(path)"
        case .notDirectory(let path):
            return "path is not a directory: \(path)"
        case .notAppBundle(let path):
            return "path is not an .app bundle: \(path)"
        case .outsideApplications(let path):
            return "path is outside /Applications: \(path)"
        case .protectedSystemPath(let path):
            return "path is protected: \(path)"
        case .photonCastSelf(let path):
            return "refusing to uninstall PhotonCast: \(path)"
        case .missingInfoPlist(let path):
            return "missing Contents/Info.plist: \(path)"
        case .missingBundleIdentifier(let path):
            return "missing CFBundleIdentifier: \(path)"
        case .appleBundle(let bundleID):
            return "refusing to uninstall Apple bundle: \(bundleID)"
        }
    }
}

func validateApplicationBundle(_ rawPath: String?) throws -> URL {
    guard let rawPath, !rawPath.isEmpty else { throw ValidationError.missingPath }
    guard rawPath.hasPrefix("/") else { throw ValidationError.relativePath(rawPath) }

    let url = URL(fileURLWithPath: rawPath).standardizedFileURL
    let canonical = url.resolvingSymlinksInPath()
    let path = canonical.path

    guard FileManager.default.fileExists(atPath: path) else {
        throw ValidationError.cannotCanonicalize(rawPath)
    }

    var isDirectory: ObjCBool = false
    guard FileManager.default.fileExists(atPath: path, isDirectory: &isDirectory), isDirectory.boolValue else {
        throw ValidationError.notDirectory(path)
    }

    guard canonical.pathExtension == "app" else { throw ValidationError.notAppBundle(path) }
    guard canonical.deletingLastPathComponent().path == "/Applications" else {
        throw ValidationError.outsideApplications(path)
    }
    guard !path.hasPrefix("/System/") else { throw ValidationError.protectedSystemPath(path) }
    guard canonical.lastPathComponent != "PhotonCast.app" else { throw ValidationError.photonCastSelf(path) }

    let infoURL = canonical.appendingPathComponent("Contents/Info.plist")
    guard FileManager.default.fileExists(atPath: infoURL.path) else {
        throw ValidationError.missingInfoPlist(path)
    }
    guard let info = NSDictionary(contentsOf: infoURL),
          let bundleID = info["CFBundleIdentifier"] as? String,
          !bundleID.isEmpty else {
        throw ValidationError.missingBundleIdentifier(path)
    }
    guard !bundleID.hasPrefix("com.apple.") else { throw ValidationError.appleBundle(bundleID) }
    guard bundleID != photonCastAppBundleID else { throw ValidationError.photonCastSelf(path) }

    return canonical
}

func encodeResponse(_ response: PrivilegedResponse) -> Data {
    (try? JSONEncoder().encode(response)) ?? Data()
}
