//! ロックファイルによる重複起動防止
//!
//! 同じディレクトリで複数のcctaktインスタンスが起動されることを防ぎます。

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

/// ロックファイルのパス（.cctakt/lock）
const LOCK_FILE_NAME: &str = ".cctakt/lock";

/// ロックファイルを管理する構造体
///
/// Drop時に自動的にロックを解放します。
pub struct LockFile {
    path: PathBuf,
}

impl LockFile {
    /// ロックを取得する
    ///
    /// 既に別のプロセスがロックを保持している場合はエラーを返します。
    /// 古いロックファイル（プロセスが終了済み）は自動的に削除されます。
    pub fn acquire() -> Result<Self> {
        let lock_path = PathBuf::from(LOCK_FILE_NAME);

        // .cctaktディレクトリを作成
        if let Some(parent) = lock_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("ディレクトリの作成に失敗: {}", parent.display()))?;
        }

        // 既存のロックファイルをチェック
        if lock_path.exists() {
            let existing_pid = Self::read_pid(&lock_path)?;

            if Self::is_process_alive(existing_pid) {
                bail!(
                    "既に別のcctaktインスタンスが実行中です (PID: {})\n\
                     同じディレクトリで複数のcctaktを起動することはできません。\n\
                     既存のインスタンスを終了してから再度お試しください。",
                    existing_pid
                );
            }

            // プロセスが終了済みなら古いロックを削除
            fs::remove_file(&lock_path)
                .with_context(|| format!("古いロックファイルの削除に失敗: {}", lock_path.display()))?;
        }

        // 新しいロックファイルを作成
        let current_pid = process::id();
        fs::write(&lock_path, current_pid.to_string())
            .with_context(|| format!("ロックファイルの作成に失敗: {}", lock_path.display()))?;

        Ok(Self { path: lock_path })
    }

    /// ロックファイルからPIDを読み取る
    fn read_pid(path: &Path) -> Result<u32> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("ロックファイルの読み取りに失敗: {}", path.display()))?;

        content
            .trim()
            .parse::<u32>()
            .with_context(|| format!("ロックファイルのPIDが無効です: {}", content.trim()))
    }

    /// 指定したPIDのプロセスが生きているかチェック
    fn is_process_alive(pid: u32) -> bool {
        // Linuxでは /proc/{pid} が存在するかで判定
        #[cfg(target_os = "linux")]
        {
            Path::new(&format!("/proc/{}", pid)).exists()
        }

        // macOSでは kill -0 で判定
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            Command::new("kill")
                .args(["-0", &pid.to_string()])
                .output()
                .map(|out| out.status.success())
                .unwrap_or(false)
        }

        // その他のOSではフォールバック
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            // プロセス存在チェックが難しい場合は、安全側に倒してtrueを返す
            // （誤って上書きしないようにする）
            true
        }
    }

    /// ロックを明示的に解放する
    pub fn release(self) {
        // dropを呼び出すだけ
        drop(self);
    }
}

impl Drop for LockFile {
    fn drop(&mut self) {
        // ロックファイルを削除
        if self.path.exists() {
            if let Err(e) = fs::remove_file(&self.path) {
                eprintln!("警告: ロックファイルの削除に失敗しました: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// テスト用にカレントディレクトリを変更してテストを実行する
    /// 他のテストと競合しないよう、完了後に元のディレクトリに戻す
    fn run_in_temp_dir<F, R>(f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let original_dir = env::current_dir().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        env::set_current_dir(&temp_dir).unwrap();

        let result = f();

        // テスト完了後に元のディレクトリに戻す
        env::set_current_dir(&original_dir).unwrap();
        result
    }

    #[test]
    #[serial]
    fn test_acquire_and_release() {
        run_in_temp_dir(|| {
            // ロックを取得
            let lock = LockFile::acquire().expect("ロック取得に失敗");

            // ロックファイルが存在することを確認
            assert!(PathBuf::from(LOCK_FILE_NAME).exists());

            // ロックを解放
            lock.release();

            // ロックファイルが削除されたことを確認
            assert!(!PathBuf::from(LOCK_FILE_NAME).exists());
        });
    }

    #[test]
    #[serial]
    fn test_stale_lock_cleanup() {
        run_in_temp_dir(|| {
            // 存在しないPIDでロックファイルを作成（古いロックをシミュレート）
            fs::create_dir_all(".cctakt").unwrap();
            fs::write(LOCK_FILE_NAME, "999999999").unwrap(); // 存在しないであろうPID

            // ロックを取得できることを確認（古いロックは削除される）
            let lock = LockFile::acquire().expect("古いロックがあっても取得できるはず");

            // 現在のPIDが書き込まれていることを確認
            let content = fs::read_to_string(LOCK_FILE_NAME).unwrap();
            assert_eq!(content, process::id().to_string());

            lock.release();
        });
    }

    #[test]
    fn test_is_process_alive_current() {
        // 現在のプロセスは生きている
        assert!(LockFile::is_process_alive(process::id()));
    }

    #[test]
    fn test_is_process_alive_nonexistent() {
        // 存在しないプロセス（十分大きなPID）
        #[cfg(target_os = "linux")]
        assert!(!LockFile::is_process_alive(999999999));
    }
}
