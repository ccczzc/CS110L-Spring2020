use linked_list::LinkedList;
pub mod linked_list;

fn main() {
    let mut list: LinkedList<String> = LinkedList::new();
    assert!(list.is_empty());
    assert_eq!(list.get_size(), 0);
    for i in 0..12 {
        list.push_front(i.to_string());
    }
    println!("{}", list);
    println!("list size: {}", list.get_size());
    println!("top element: {}", list.pop_front().unwrap());
    println!("list: {}", list);
    println!("size: {}", list.get_size());
    println!("list: {}", list.to_string()); // ToString impl for anything impl Display
    let mut cloned_list = list.clone();
    println!("cloned list: {}", cloned_list);
    println!("'list = cloned list': {}", list == cloned_list);
    cloned_list.push_front(String::from("12"));
    println!("now, list: {}", list);
    println!("now, cloned list: {}", cloned_list);
    println!("'list != cloned list': {}", list != cloned_list);
    // Implementing IntoIterator for &LinkedList<T>
    for val in &list {
        println!("{}", val);
    }
    // The iterator will take ownership of the LinkedList<T> lsit
    for val in list {
        println!("{}", val);
    }
}
