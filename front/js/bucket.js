// bucket.js

const FILE_ACTIONS = {
    'Open 🡽': file_new_tab,
    'Delete': file_delete,
};

function size_fmt(num_bytes) {
    let digits = num_bytes.toString().length;
    /**/ if (digits > 9) return parseInt(num_bytes / 10**9).toString() + ' GB';
    else if (digits > 6) return parseInt(num_bytes / 10**6).toString() + ' MB';
    else if (digits > 3) return parseInt(num_bytes / 10**3).toString() + ' kB';
    else return num_bytes.toString() + ' B';
}

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
        let file_c = ['btn', 'border1-c2-bottom', 'flex-h'];
        let file_e = create(side.files_div, 'div', file_c);
        file_e.index = i;

        let file_name = create(file_e, 'span', ['pad05', 'grow', 'border2-c2-right']);
        let file_sz = create(file_e, 'span', ['pad05', 'border2-c2-right', 'w4', 'ta-center']);
        let file_up = create(file_e, 'span', ['pad05', 'w9', 'ta-center']);

        file_name.innerText = file.name;
        file_sz.innerText = size_fmt(file.size);
        file_up.innerText = datetime_string(file.uploaded);

        init_context_menu(file_e, FILE_ACTIONS);
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
        

        if (file.size > USER_DATA.secret.max_file_size) {
            alert(file.name + ': fichier trop volumineux.');
            continue;
        }

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
