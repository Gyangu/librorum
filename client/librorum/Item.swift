//
//  Item.swift
//  librorum
//
//  Created by 戈洋 on 2025/6/23.
//

import Foundation
import SwiftData

@Model
final class Item {
    var timestamp: Date
    
    init(timestamp: Date) {
        self.timestamp = timestamp
    }
}
