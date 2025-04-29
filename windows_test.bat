@echo off
rem Windows 平台自动化通信测试脚本

rem 运行单元测试
 echo [windows_test] 在 Windows 上运行单元测试
 cargo test

rem 停止可能已经运行的守护进程
 echo [windows_test] 停止可能已运行的守护进程
 .\target\debug\librorum.exe stop || echo 忽略错误，继续

rem 启动守护进程
 echo [windows_test] 启动守护进程
 start /B .\target\debug\librorum.exe start

rem 等待服务启动
 timeout /t 3 >nul

rem 获取最新日志
 echo [windows_test] 获取最新日志
 .\target\debug\librorum.exe logs --tail 10 | more

rem 测试连接 Mac 节点 - 使用直接测试模式
 echo [windows_test] 测试直接连接 Mac 节点: gy.local:50051
 .\target\debug\librorum.exe test-connect gy.local:50051 || echo 连接失败，尝试备用IP地址
 
rem 尝试连接备用IP地址
 echo [windows_test] 尝试连接备用Mac IP: 192.168.31.90:50051
 .\target\debug\librorum.exe test-connect 192.168.31.90:50051 || echo 备用IP连接也失败

rem 停止守护进程
 echo [windows_test] 停止守护进程
 .\target\debug\librorum.exe stop

 echo [windows_test] Windows 平台通信测试完成 