import Foundation
import SwiftData

class MockDataService {
    static let shared = MockDataService()
    
    private init() {}
    
    // 生成模拟文件列表
    func generateMockFiles() -> [FileItem] {
        let rootFolder = FileItem(
            name: "我的文件",
            isDirectory: true,
            path: "/root"
        )
        
        let documentsFolder = FileItem(
            name: "文档",
            isDirectory: true,
            path: "/root/documents",
            parentId: rootFolder.id
        )
        
        let photosFolder = FileItem(
            name: "照片",
            isDirectory: true,
            path: "/root/photos",
            parentId: rootFolder.id
        )
        
        let projectsFolder = FileItem(
            name: "项目",
            isDirectory: true,
            path: "/root/projects",
            parentId: rootFolder.id
        )
        
        // 添加一些文档
        let wordDoc = FileItem(
            name: "会议记录.docx",
            isDirectory: false,
            size: 1024 * 25,
            path: "/root/documents/meeting.docx",
            parentId: documentsFolder.id
        )
        
        let pdfDoc = FileItem(
            name: "报告.pdf",
            isDirectory: false,
            size: 1024 * 1024 * 2,
            path: "/root/documents/report.pdf",
            parentId: documentsFolder.id,
            syncStatus: .pendingSync
        )
        
        let excelDoc = FileItem(
            name: "财务表格.xlsx",
            isDirectory: false,
            size: 1024 * 512,
            path: "/root/documents/financial.xlsx",
            parentId: documentsFolder.id
        )
        
        // 添加一些照片
        let photo1 = FileItem(
            name: "假期照片.jpg",
            isDirectory: false,
            size: 1024 * 1024 * 5,
            path: "/root/photos/vacation.jpg",
            parentId: photosFolder.id
        )
        
        let photo2 = FileItem(
            name: "家庭合照.png",
            isDirectory: false,
            size: 1024 * 1024 * 3,
            path: "/root/photos/family.png",
            parentId: photosFolder.id,
            syncStatus: .syncing
        )
        
        // 添加一些项目文件
        let projectFile = FileItem(
            name: "项目计划.md",
            isDirectory: false,
            size: 1024 * 15,
            path: "/root/projects/plan.md",
            parentId: projectsFolder.id
        )
        
        let codeFile = FileItem(
            name: "源代码.zip",
            isDirectory: false,
            size: 1024 * 1024 * 10,
            path: "/root/projects/source.zip",
            parentId: projectsFolder.id,
            syncStatus: .conflicted
        )
        
        return [
            rootFolder, documentsFolder, photosFolder, projectsFolder,
            wordDoc, pdfDoc, excelDoc, photo1, photo2, projectFile, codeFile
        ]
    }
    
    // 生成模拟同步历史
    func generateMockSyncHistory() -> [SyncHistory] {
        return [
            SyncHistory(
                timestamp: Date().addingTimeInterval(-86400 * 3),
                status: .synced,
                details: "完成初始同步",
                fileCount: 120,
                totalSize: 1024 * 1024 * 1024 * 2
            ),
            SyncHistory(
                timestamp: Date().addingTimeInterval(-86400 * 2),
                status: .error,
                details: "网络连接中断",
                fileCount: 5,
                totalSize: 1024 * 1024 * 50
            ),
            SyncHistory(
                timestamp: Date().addingTimeInterval(-86400),
                status: .synced,
                details: "增量同步完成",
                fileCount: 12,
                totalSize: 1024 * 1024 * 150
            ),
            SyncHistory(
                timestamp: Date().addingTimeInterval(-3600),
                status: .syncing,
                details: "当前同步进行中",
                fileCount: 8,
                totalSize: 1024 * 1024 * 75
            )
        ]
    }
    
    // 模拟用户设置
    func generateMockUserPreferences() -> UserPreferences {
        return UserPreferences(
            syncFrequency: .daily,
            autoSync: true,
            defaultSavePath: "~/Documents/Librorum",
            syncOnCellular: false,
            darkModeEnabled: true,
            lastSyncDate: Date().addingTimeInterval(-3600)
        )
    }
} 