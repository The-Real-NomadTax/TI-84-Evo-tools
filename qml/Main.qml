import QtQuick
import QtQuick.Controls as Controls
import QtQuick.Dialogs
import QtQuick.Layouts
import org.example.tiscreen

Controls.ApplicationWindow {
    id: root
    width: 760
    height: 640
    minimumWidth: 420
    minimumHeight: 360
    visible: true
    title: qsTr("TI Calculator")

    CaptureBackend { id: backend }

    function openPage(page) {
        pageStack.replace(page)
    }

    function navigate(index) {
        navigationTabs.currentIndex = index
        switch (index) {
        case 0:
            openPage(homePage)
            break
        case 1:
            openPage(screenPage)
            break
        case 2:
            openPage(filesPage)
            break
        case 3:
            openPage(sendPage)
            break
        }
    }

    header: Controls.ToolBar {
        RowLayout {
            anchors.fill: parent

            Controls.ToolButton {
                text: qsTr("Navigation")
                onClicked: navigationDrawer.open()
            }

            Controls.Label {
                Layout.fillWidth: true
                text: [qsTr("Main Menu"), qsTr("Screen Capture"),
                       qsTr("Device Files"), qsTr("Send Files")][navigationTabs.currentIndex]
                font.bold: true
            }
        }
    }

    Controls.Drawer {
        id: navigationDrawer
        width: Math.min(root.width * 0.8, 320)
        height: root.height
        edge: Qt.LeftEdge

        ColumnLayout {
            anchors.fill: parent
            anchors.margins: 16
            spacing: 8

            Controls.Label {
                Layout.fillWidth: true
                text: qsTr("TI Calculator")
                font.pixelSize: 22
                font.bold: true
            }

            Controls.Button {
                Layout.fillWidth: true
                text: qsTr("Main Menu")
                onClicked: {
                    root.navigate(0)
                    navigationDrawer.close()
                }
            }

            Controls.Button {
                Layout.fillWidth: true
                text: qsTr("Screen Capture")
                onClicked: {
                    root.navigate(1)
                    navigationDrawer.close()
                }
            }

            Controls.Button {
                Layout.fillWidth: true
                text: qsTr("Device Files")
                onClicked: {
                    root.navigate(2)
                    navigationDrawer.close()
                }
            }

            Controls.Button {
                Layout.fillWidth: true
                text: qsTr("Send Files")
                onClicked: {
                    root.navigate(3)
                    navigationDrawer.close()
                }
            }
        }
    }

    Controls.StackView {
        id: pageStack
        anchors.fill: parent
        initialItem: homePage
    }

    footer: Controls.TabBar {
        id: navigationTabs
        currentIndex: 0

        Controls.TabButton {
            text: qsTr("Menu")
            icon.name: "go-home"
            onClicked: root.navigate(0)
        }

        Controls.TabButton {
            text: qsTr("Capture")
            icon.name: "camera-photo"
            onClicked: root.navigate(1)
        }

        Controls.TabButton {
            text: qsTr("Files")
            icon.name: "folder"
            onClicked: root.navigate(2)
        }

        Controls.TabButton {
            text: qsTr("Send")
            icon.name: "document-send"
            onClicked: root.navigate(3)
        }
    }

    Component {
        id: homePage

        Controls.Page {
            title: qsTr("Main Menu")

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 32, 448)
                spacing: 16

                Controls.Label {
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("TI Calculator Tools")
                    font.pixelSize: 28
                    font.bold: true
                }

                Controls.Button {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 56
                    text: qsTr("Screen Capture")
                    icon.name: "camera-photo"
                    onClicked: root.navigate(1)
                }

                Controls.Label {
                    Layout.fillWidth: true
                    text: qsTr("Capture the calculator display and save it as a PNG.")
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Controls.Button {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 56
                    text: qsTr("Check Device Files")
                    icon.name: "folder"
                    enabled: !backend.busy
                    onClicked: {
                        root.navigate(2)
                        backend.refreshFiles()
                    }
                }

                Controls.Label {
                    Layout.fillWidth: true
                    text: qsTr("Read programs, lists, images, and other variables from the device directory.")
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Controls.Button {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 56
                    text: qsTr("Send a File")
                    icon.name: "document-send"
                    enabled: !backend.busy
                    onClicked: root.navigate(3)
                }

                Controls.Label {
                    Layout.fillWidth: true
                    text: qsTr("Choose a TI variable file on this computer and transfer it to the calculator.")
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }
            }
        }
    }

    Component {
        id: screenPage

        Controls.Page {
            id: capturePage
            title: qsTr("Screen Capture")

            Timer {
                id: autoCaptureTimer
                interval: autoCaptureInterval.value * 10
                running: autoCaptureSwitch.checked && capturePage.visible
                repeat: true
                onTriggered: {
                    if (!backend.busy) {
                        backend.capture(cursorCheck.checked)
                    }
                }
            }

            ColumnLayout {
                anchors.fill: parent
                spacing: 16

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: root.palette.alternateBase
                    radius: 4

                    Image {
                        anchors.fill: parent
                        anchors.margins: 16
                        source: backend.imageUrl
                        fillMode: Image.PreserveAspectFit
                        cache: false
                        smooth: false
                        visible: source.toString().length > 0
                    }

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width - 64
                        visible: backend.imageUrl.length === 0 && !backend.busy

                        Controls.Label {
                            Layout.fillWidth: true
                            text: qsTr("No capture yet")
                            font.pixelSize: 20
                            font.bold: true
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Controls.Label {
                            Layout.fillWidth: true
                            text: qsTr("Connect a supported TI calculator over USB.")
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }

                    Controls.BusyIndicator {
                        anchors.centerIn: parent
                        running: backend.busy && !autoCaptureSwitch.checked
                        visible: running
                    }
                }

                RowLayout {
                    Layout.fillWidth: true

                    Controls.CheckBox {
                        id: cursorCheck
                        text: qsTr("Include cursor")
                        enabled: !backend.busy
                    }

                    Item { Layout.fillWidth: true }

                    Controls.Button {
                        text: qsTr("Capture Screen")
                        icon.name: "camera-photo"
                        enabled: !backend.busy
                        onClicked: backend.capture(cursorCheck.checked)
                    }
                }

                RowLayout {
                    Layout.fillWidth: true
                    spacing: 16

                    Controls.Switch {
                        id: autoCaptureSwitch
                        text: qsTr("Auto capture")
                    }

                    Item { Layout.fillWidth: true }

                    Controls.Label {
                        text: qsTr("Interval:")
                    }

                    Controls.SpinBox {
                        id: autoCaptureInterval
                        from: 1
                        to: 30000
                        value: 500
                        stepSize: 25
                        editable: true
                        enabled: autoCaptureSwitch.checked
                        Accessible.name: qsTr("Auto-capture interval in seconds")

                        textFromValue: function(value, locale) {
                            return Number(value / 100).toLocaleString(locale, "f", 2)
                        }

                        valueFromText: function(text, locale) {
                            return Math.round(Number.fromLocaleString(locale, text) * 100)
                        }
                    }

                    Controls.Label {
                        text: qsTr("seconds")
                    }
                }

                Controls.Label {
                    Layout.fillWidth: true
                    visible: autoCaptureSwitch.checked
                    text: qsTr("Very fast capture intervals, especially below 0.25 seconds, may cause connection issues or make the calculator temporarily unresponsive.")
                    wrapMode: Text.WordWrap
                }

                Controls.Label {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: backend.status
                    wrapMode: Text.WordWrap
                    color: backend.status.indexOf("failed") >= 0
                        ? "#b00020"
                        : root.palette.text
                }
            }
        }
    }

    Component {
        id: filesPage

        Controls.Page {
            id: deviceFilesPage
            title: qsTr("Device Files")
            property var deviceFiles: []

            function loadDeviceFiles() {
                try {
                    deviceFiles = JSON.parse(backend.filesJson)
                } catch (error) {
                    deviceFiles = []
                }
            }

            Connections {
                target: backend
                function onFilesJsonChanged() {
                    deviceFilesPage.loadDeviceFiles()
                }
            }

            Controls.Dialog {
                id: deleteDialog
                anchors.centerIn: parent
                modal: true
                title: qsTr("Delete device file?")
                standardButtons: Controls.Dialog.Ok | Controls.Dialog.Cancel
                property var pendingFile: null

                contentItem: Controls.Label {
                    width: 352
                    text: deleteDialog.pendingFile
                        ? qsTr("Permanently delete %1 (type %2) from the calculator?")
                            .arg(deleteDialog.pendingFile.name)
                            .arg(deleteDialog.pendingFile.type)
                        : ""
                    wrapMode: Text.WordWrap
                }

                onAccepted: {
                    if (pendingFile) {
                        backend.deleteFile(pendingFile.name,
                                           pendingFile.wireName,
                                           pendingFile.type)
                    }
                    pendingFile = null
                }
                onRejected: pendingFile = null
            }

            ColumnLayout {
                anchors.fill: parent
                spacing: 16

                Controls.Button {
                    Layout.alignment: Qt.AlignRight
                    text: qsTr("Refresh")
                    enabled: !backend.busy
                    onClicked: backend.refreshFiles()
                }

                Rectangle {
                    Layout.fillWidth: true
                    Layout.fillHeight: true
                    color: root.palette.alternateBase
                    radius: 4

                    ListView {
                        id: deviceFileList
                        anchors.fill: parent
                        anchors.margins: 8
                        clip: true
                        spacing: 8
                        model: deviceFilesPage.deviceFiles

                        delegate: Controls.ItemDelegate {
                            required property var modelData
                            width: ListView.view.width
                            enabled: !backend.busy

                            contentItem: RowLayout {
                                spacing: 16

                                ColumnLayout {
                                    Layout.fillWidth: true
                                    spacing: 0

                                    Controls.Label {
                                        Layout.fillWidth: true
                                        text: modelData.name
                                        font.bold: true
                                        elide: Text.ElideRight
                                    }

                                    Controls.Label {
                                        Layout.fillWidth: true
                                        text: qsTr("%1 bytes · type %2 · version %3 · %4")
                                            .arg(modelData.size)
                                            .arg(modelData.type)
                                            .arg(modelData.version)
                                            .arg(modelData.memory ? qsTr("archived") : qsTr("RAM"))
                                        opacity: 0.75
                                        elide: Text.ElideRight
                                    }
                                }

                                Controls.Button {
                                    text: qsTr("Delete")
                                    icon.name: "edit-delete"
                                    enabled: modelData.wireName.length > 0 && !backend.busy
                                    onClicked: {
                                        deleteDialog.pendingFile = modelData
                                        deleteDialog.open()
                                    }
                                }
                            }
                        }

                        Controls.ScrollBar.vertical: Controls.ScrollBar {}
                    }

                    ColumnLayout {
                        anchors.centerIn: parent
                        width: parent.width - 64
                        visible: deviceFileList.count === 0 && !backend.busy

                        Controls.Label {
                            Layout.fillWidth: true
                            text: qsTr("No device files loaded")
                            font.pixelSize: 20
                            font.bold: true
                            horizontalAlignment: Text.AlignHCenter
                        }

                        Controls.Label {
                            Layout.fillWidth: true
                            text: qsTr("Press Refresh to read files from the connected calculator.")
                            wrapMode: Text.WordWrap
                            horizontalAlignment: Text.AlignHCenter
                        }
                    }
                }

                Controls.BusyIndicator {
                    Layout.alignment: Qt.AlignHCenter
                    running: backend.busy
                    visible: running
                }

                Controls.Label {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: backend.status
                    wrapMode: Text.WordWrap
                    color: backend.status.indexOf("failed") >= 0
                        ? "#b00020"
                        : root.palette.text
                }
            }
        }
    }

    Component {
        id: sendPage

        Controls.Page {
            id: sendFilePage
            title: qsTr("Send Files")
            property string selectedPath: ""

            FileDialog {
                id: fileDialog
                title: qsTr("Choose a TI calculator file")
                fileMode: FileDialog.OpenFile
                nameFilters: [
                    qsTr("TI-84 EVO programs (*.8xp2)"),
                    qsTr("Python programs (*.py)"),
                    qsTr("All files (*)")
                ]
                onAccepted: {
                    sendFilePage.selectedPath = decodeURIComponent(
                        selectedFile.toString().replace(/^file:\/\//, ""))
                }
            }

            ColumnLayout {
                anchors.centerIn: parent
                width: Math.min(parent.width - 32, 480)
                spacing: 16

                Controls.Label {
                    Layout.alignment: Qt.AlignHCenter
                    text: qsTr("Transfer to Calculator")
                    font.pixelSize: 24
                    font.bold: true
                }

                Controls.Label {
                    Layout.fillWidth: true
                    text: qsTr("Select a native TI-84 EVO .8xp2 program or a Python .py file. Python filenames must contain 1–8 letters.")
                    wrapMode: Text.WordWrap
                    horizontalAlignment: Text.AlignHCenter
                }

                Controls.Button {
                    Layout.fillWidth: true
                    text: qsTr("Choose File…")
                    icon.name: "document-open"
                    enabled: !backend.busy
                    onClicked: fileDialog.open()
                }

                Controls.TextField {
                    Layout.fillWidth: true
                    text: sendFilePage.selectedPath
                    readOnly: true
                    placeholderText: qsTr("No host file selected")
                }

                Controls.Button {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 48
                    text: qsTr("Send to Device")
                    icon.name: "document-send"
                    enabled: sendFilePage.selectedPath.length > 0 && !backend.busy
                    onClicked: backend.uploadFile(sendFilePage.selectedPath)
                }

                Controls.BusyIndicator {
                    Layout.alignment: Qt.AlignHCenter
                    running: backend.busy
                    visible: running
                }

                Controls.Label {
                    Layout.fillWidth: true
                    visible: text.length > 0
                    text: backend.status
                    wrapMode: Text.WordWrap
                    color: backend.status.indexOf("failed") >= 0
                        ? "#b00020"
                        : root.palette.text
                }
            }
        }
    }
}
