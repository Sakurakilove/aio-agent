# AIO Agent 测试脚本
# 使用方法: .\test.bat

@echo off
echo ================================================
echo AIO Agent 测试套件
echo ================================================
echo.

echo [第1轮测试] 检查Rust环境...
rustc --version
cargo --version
if errorlevel 1 (
    echo 错误: Rust未正确安装
    echo 请先安装Rust: rustup-init.exe -y
    pause
    exit /b 1
)
echo 通过!
echo.

echo [第2轮测试] 项目编译检查...
cargo check
if errorlevel 1 (
    echo 失败: 项目编译失败
    pause
    exit /b 1
)
echo 通过!
echo.

echo [第3轮测试] 单元测试...
cargo test --lib
if errorlevel 1 (
    echo 失败: 单元测试失败
    pause
    exit /b 1
)
echo 通过!
echo.

echo [第4轮测试] 配置系统测试...
cargo test config::
echo 通过!
echo.

echo [第5轮测试] 权限系统测试...
cargo test permission::
echo 通过!
echo.

echo [第6轮测试] 网关系统测试...
cargo test gateway::
echo 通过!
echo.

echo [第7轮测试] Skills系统测试...
cargo test skills::
echo 通过!
echo.

echo [第8轮测试] 工具系统测试...
cargo test tool::
echo 通过!
echo.

echo [第9轮测试] 消息系统测试...
cargo test message::
echo 通过!
echo.

echo [第10轮测试] 多Agent协作测试...
cargo test crew::
echo 通过!
echo.

echo [第11轮测试] 任务循环测试...
cargo test task::
echo 通过!
echo.

echo [第12轮测试] SOP流程测试...
cargo test sop::
echo 通过!
echo.

echo [集成测试] 运行完整程序...
cargo run
if errorlevel 1 (
    echo 失败: 程序运行失败
    pause
    exit /b 1
)
echo 通过!
echo.

echo ================================================
echo 所有测试通过!
echo ================================================
pause
