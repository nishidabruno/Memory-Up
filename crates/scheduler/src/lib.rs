use domain::repositories::{DecksRepository, FlashcardsRepository};
use futures::future::join_all;
use sql::{DbManager, DecksRepositorySql, FlashcardsRepositorySql};
use tauri::AppHandle;
use tauri_plugin_notification::{NotificationExt, PermissionState};
use tokio::time::{interval, Duration};
use utils::AppError;

pub async fn setup_scheduler(app: AppHandle) -> Result<(), AppError> {
    let db_manager = DbManager::new().await.unwrap();
    let pool = db_manager.get_db_instance();

    let interval_time = Duration::from_secs(60 * 60 * 2); // 2 hours

    let mut interval = interval(interval_time);
    interval.tick().await;

    loop {
        interval.tick().await;
        let decks_repository = DecksRepositorySql::new(pool.clone());
        let flashcards_repository = FlashcardsRepositorySql::new(pool.clone());
        let decks = decks_repository.list_all().await?;

        let futures: Vec<_> = decks
            .iter()
            .map(|deck| async {
                flashcards_repository
                    .find_flashcard_for_review(deck.id)
                    .await
            })
            .collect();

        let results = join_all(futures).await;

        let has_due_flashcard = results
            .iter()
            .any(|flashcard| matches!(flashcard, Ok(Some(_))));

        if has_due_flashcard {
            let notify = app.notification();
            if notify.permission_state().unwrap() != PermissionState::Granted {
                notify.request_permission().unwrap();
            }
            if notify.permission_state().unwrap() == PermissionState::Granted {
                notify
                    .builder()
                    .title("Memory Up")
                    .body("Vamos estudar? Flashcards estão disponíveis para revisão!")
                    .show()
                    .unwrap();
            }
        }
    }
}
