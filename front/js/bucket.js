// bucket.js

async function init_bucket(side_i) {
    let side = SIDES[side_i];
    let [_, [rev, files]] = await request('load-bucket', side.raw_id);
    side.revision = rev;
    side.files = files;

    await init_banner(side_i);

    let div_c = ['grow', 'flex-v', 'autoverflow'];
    side.files_div = create(side.element, 'div', div_c);
    side.files_div.side_i = side_i;

    let actions = { 'Upload': upload, };
    init_context_menu(side.files_div, actions);

    for (let i = 0; i < files.length; i++) {
        let file = side.files[i];
        let file_c = ['pad05', 'btn', 'border1-c2-bottom'];
        let file_e = create(side.files_div, 'div', file_c);
        file_e.innerText = file.name;
        file_e.index = i;

        let actions = {
            'Open 🡽': file_new_tab,
            'Delete': file_delete,
        };

        init_context_menu(file_e, actions);
    }
}

async function file_new_tab() {
    let side = SIDES[this.parentElement.side_i];
    let data = side.files[this.index];
    let anchor = document.createElement('a');
    anchor.href = '/files/' + data.sha256 + '.dat';
    anchor.target = '_blank';
    anchor.click();
}

async function file_delete() {
    if (!confirm('Delete this file?')) return;
    let side = SIDES[this.parentElement.side_i];
    let parameters = [side.raw_id, side.revision, this.index];
    let _ = await request('delete-file', parameters);
}

async function upload() {
    let file_input = document.createElement('input');
    file_input.type = 'file';
    file_input.side_i = this.side_i;
    file_input.addEventListener('change', file_pick);
    file_input.click();
}

async function file_pick() {
    find('upload-popup').classList.remove('hidden');
    COVER.classList.remove('hidden');
    CAN_CLOSE_POPUP = false;

    let side = SIDES[this.side_i];
    for (let i = 0; i < this.files.length; i++) {
        let file = this.files[i];
        console.log(file);

        for (let j = 0; j < file.size; j += CHUNK) {
            let chunk = file.slice(j, j + CHUNK);
            SOCKET.send(chunk);
        }

        let parameters = [side.raw_id, side.revision, file.name];
        let _ = await request('finish-file', parameters);
    }

    CAN_CLOSE_POPUP = true;
    find('upload-popup').classList.add('hidden');
    COVER.classList.add('hidden');
}
