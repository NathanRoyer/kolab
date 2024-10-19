// init.js

function ask_for_notifications() {
    Notification.requestPermission();
}

async function try_auto_login() {
    let success = false;
    if (localStorage['user-id']) {
        USER_ID = parseInt(localStorage['user-id']);
        try {
            await login_auth();
            success = true;
        } catch {
            USER_ID = undefined;
            delete localStorage['user-id'];
            console.warn("outdated token");
        }
    }

    if (success) {
        CAN_CLOSE_POPUP = true;
        COVER.click();
    } else {
        let login_form = find('login-form');
        login_form.classList.remove('hidden');
        find('username-input').focus();
        setHandler('auth-button', 'click', ask_for_notifications);
    }
}

function init() {
    if (localStorage['theme']) {
        document.body.id = localStorage['theme'];
    }

    SIDES = [{ element: find('view-a') }, {}];
    LEFT_PANEL_ITEMS = find('left-panel-items');
    CONTEXT_MENU = find('context-menu');
    MAIN_PANEL = find('main-panel');
    COVER = find('cover');
    COVER.addEventListener('click', close_popup);

    redirect_enter(find('username-input'), find('password-input'));
    redirect_enter(find('password-input'), find('auth-button'));

    setHandler('auth-button', 'click', login_or_create_account);
    init_selectable([find('login-button'), find('create-account')]);

    setHandler('collapse-button', 'click', collapse_click);
    setHandler('user-btn', 'click', user_click);
    setHandler('list-invites', 'click', show_invites_list);
    setHandler('context-menu-background', 'click', hide_context_menu);

    let create_actions = {
        'Conversation': create_entity,
        'Document': create_entity,
        // 'Spreadsheet': create_entity,
        'Bucket': create_entity,
    };

    init_context_menu(find('create-entity'), create_actions);

    init_websocket();
}

window.addEventListener("load", init);
